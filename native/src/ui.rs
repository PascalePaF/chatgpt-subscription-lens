use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use chrono::{DateTime, Local};
use eframe::egui::{
    self, pos2, vec2, Align, Align2, Color32, CornerRadius, FontData, FontDefinitions, FontFamily,
    FontId, ImageData, Layout, Margin, Rect, RichText, Sense, Stroke, StrokeKind, TextureHandle,
    TextureOptions, UiBuilder, Vec2, ViewportBuilder,
};
use zeroize::Zeroize;

use crate::models::{
    AccountIdentity, BillingRecord, CoverageInfo, CredentialValidation, InspectionRequest,
    InspectionResult, PaymentMethodSummary, SubscriptionSummary, UsageSummary, UsageWindow,
};
use crate::{inspect_subscription, validate_credential, AppError};

const WINDOW_SIZE: Vec2 = vec2(1280.0, 800.0);
const SIDEBAR_WIDTH: f32 = 184.0;
const CREDENTIAL_EDITOR_HEIGHT: f32 = 76.0;
const PAGE_BG: Color32 = Color32::from_rgb(245, 244, 239);
const SIDEBAR_BG: Color32 = Color32::from_rgb(239, 237, 230);
const INK: Color32 = Color32::from_rgb(37, 36, 32);
const MUTED: Color32 = Color32::from_rgb(112, 108, 99);
const BORDER: Color32 = Color32::from_rgb(218, 214, 204);
const ACCENT: Color32 = Color32::from_rgb(198, 91, 63);
const SUCCESS: Color32 = Color32::from_rgb(47, 125, 89);
const DANGER: Color32 = Color32::from_rgb(172, 66, 52);

pub fn run_native_app() -> eframe::Result {
    let icon = load_icon(include_bytes!("../icons/icon.png"));
    let viewport = ViewportBuilder::default()
        .with_title("订阅镜 · ChatGPT 订阅查询")
        .with_inner_size(WINDOW_SIZE)
        .with_min_inner_size(WINDOW_SIZE)
        .with_max_inner_size(WINDOW_SIZE)
        .with_resizable(false)
        .with_icon(Arc::new(icon));
    let options = eframe::NativeOptions {
        viewport,
        centered: true,
        persist_window: false,
        ..Default::default()
    };

    eframe::run_native(
        "订阅镜",
        options,
        Box::new(|creation_context| Ok(Box::new(SubscriptionLensApp::new(creation_context)))),
    )
}

fn load_icon(bytes: &[u8]) -> egui::IconData {
    let image = image::load_from_memory(bytes)
        .expect("embedded application icon must decode")
        .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

struct PaymentTextures {
    visa: TextureHandle,
    mastercard: TextureHandle,
    app_store: TextureHandle,
    google_play: TextureHandle,
}

impl PaymentTextures {
    fn load(ctx: &egui::Context) -> Self {
        Self {
            visa: load_texture(
                ctx,
                "payment-visa-3d",
                include_bytes!("../../assets/payment/visa-3d.png"),
            ),
            mastercard: load_texture(
                ctx,
                "payment-mastercard-3d",
                include_bytes!("../../assets/payment/mastercard-3d.png"),
            ),
            app_store: load_texture(
                ctx,
                "payment-app-store-3d",
                include_bytes!("../../assets/payment/app-store-3d.png"),
            ),
            google_play: load_texture(
                ctx,
                "payment-google-play-3d",
                include_bytes!("../../assets/payment/google-play-3d.png"),
            ),
        }
    }
}

fn load_texture(ctx: &egui::Context, name: &str, bytes: &[u8]) -> TextureHandle {
    let image = image::load_from_memory(bytes)
        .unwrap_or_else(|_| panic!("embedded texture {name} must decode"))
        .into_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let pixels = image.into_raw();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
    ctx.load_texture(
        name,
        ImageData::Color(color_image.into()),
        TextureOptions::LINEAR,
    )
}

enum QueryMessage {
    Finished(Result<InspectionResult, AppError>),
}

struct SubscriptionLensApp {
    credential: String,
    validation: CredentialValidation,
    authorized: bool,
    querying: bool,
    query_started: Option<Instant>,
    last_input_change: Option<Instant>,
    receiver: Option<mpsc::Receiver<QueryMessage>>,
    result: Option<InspectionResult>,
    error: Option<String>,
    textures: PaymentTextures,
}

impl SubscriptionLensApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_fonts(&cc.egui_ctx);
        configure_style(&cc.egui_ctx);
        let textures = PaymentTextures::load(&cc.egui_ctx);
        let result = std::env::args()
            .any(|argument| argument == "--demo")
            .then(demo_result);
        Self {
            credential: String::new(),
            validation: validate_credential(""),
            authorized: false,
            querying: false,
            query_started: None,
            last_input_change: None,
            receiver: None,
            result,
            error: None,
            textures,
        }
    }

    fn start_query(&mut self, ctx: &egui::Context) {
        if self.querying || !self.authorized || !self.validation.can_query {
            return;
        }

        let credential = self.credential.clone();
        let timezone_offset_min = -(Local::now().offset().local_minus_utc() / 60);
        let request = InspectionRequest {
            credential,
            timezone_offset_min: Some(timezone_offset_min),
        };
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        self.querying = true;
        self.query_started = Some(Instant::now());
        self.error = None;
        let repaint = ctx.clone();

        std::thread::spawn(move || {
            let outcome = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|error| AppError::Client(error.to_string()))
                .and_then(|runtime| runtime.block_on(inspect_subscription(request)));
            let _ = sender.send(QueryMessage::Finished(outcome));
            repaint.request_repaint();
        });
    }

    fn poll_query(&mut self) {
        let message = self
            .receiver
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        if let Some(QueryMessage::Finished(outcome)) = message {
            self.querying = false;
            self.query_started = None;
            self.receiver = None;
            match outcome {
                Ok(result) => {
                    self.credential.zeroize();
                    self.credential.clear();
                    self.validation = validate_credential("");
                    self.authorized = false;
                    self.last_input_change = None;
                    self.result = Some(result);
                    self.error = None;
                }
                Err(error) => {
                    self.error = Some(error.to_string());
                }
            }
        }
    }

    fn reset(&mut self) {
        self.credential.zeroize();
        self.credential.clear();
        self.validation = validate_credential("");
        self.authorized = false;
        self.last_input_change = None;
        self.result = None;
        self.error = None;
    }

    fn draw_sidebar(&mut self, ui: &mut egui::Ui) {
        let sidebar_rect = ui.max_rect();
        ui.painter().rect_filled(sidebar_rect, 0.0, SIDEBAR_BG);
        ui.set_min_width(SIDEBAR_WIDTH);
        ui.set_max_width(SIDEBAR_WIDTH);
        ui.add_space(24.0);

        ui.horizontal(|ui| {
            let (mark_rect, _) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::hover());
            ui.painter().rect_filled(mark_rect, 10.0, INK);
            ui.painter().text(
                mark_rect.center(),
                Align2::CENTER_CENTER,
                "镜",
                FontId::proportional(17.0),
                Color32::WHITE,
            );
            ui.vertical(|ui| {
                ui.label(RichText::new("订阅镜").size(17.0).strong().color(INK));
                ui.label(
                    RichText::new(format!("V{}", env!("CARGO_PKG_VERSION")))
                        .size(10.0)
                        .color(MUTED),
                );
            });
        });

        ui.add_space(36.0);
        sidebar_item(ui, "01", "连接账户", self.result.is_none());
        ui.add_space(8.0);
        sidebar_item(ui, "02", "订阅卡片", self.result.is_some());
        ui.add_space(8.0);
        sidebar_item(ui, "03", "额度与账单", self.result.is_some());

        let status_y = sidebar_rect.bottom() - 46.0;
        ui.painter().circle_filled(
            pos2(sidebar_rect.left() + 16.0, status_y),
            3.5,
            Color32::from_rgb(69, 160, 113),
        );
        ui.painter().text(
            pos2(sidebar_rect.left() + 28.0, status_y),
            Align2::LEFT_CENTER,
            "本机只读",
            FontId::proportional(11.5),
            INK,
        );
        ui.painter().text(
            pos2(sidebar_rect.left() + 16.0, status_y + 19.0),
            Align2::LEFT_CENTER,
            "仅连接 chatgpt.com",
            FontId::proportional(10.0),
            MUTED,
        );
    }

    fn draw_query(&mut self, ui: &mut egui::Ui) {
        header_row(ui, "连接自己的 ChatGPT 账户", "凭证不写入磁盘");
        ui.add_space(18.0);

        let panel_width = ui.available_width();
        let panel_height = 620.0;
        let (panel, _) = ui.allocate_exact_size(vec2(panel_width, panel_height), Sense::hover());
        ui.painter().rect(
            panel,
            CornerRadius::same(22),
            Color32::from_rgb(250, 249, 245),
            Stroke::new(1.0, BORDER),
            StrokeKind::Inside,
        );
        let inner = panel.shrink2(vec2(42.0, 34.0));
        ui.scope_builder(UiBuilder::new().max_rect(inner), |ui| {
            ui.set_width(inner.width());
            ui.label(
                RichText::new("Session 完整性检查")
                    .size(27.0)
                    .strong()
                    .color(INK),
            );
            ui.add_space(5.0);
            ui.label(
                RichText::new(
                    "粘贴完整 Session JSON、Access Token、Codex auth.json 或 session token。",
                )
                .size(13.0)
                .color(MUTED),
            );
            ui.add_space(22.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("会话数据").size(13.0).strong().color(INK));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if !self.credential.is_empty()
                        && ui
                            .button(RichText::new("清空").size(12.0).color(MUTED))
                            .clicked()
                    {
                        self.credential.zeroize();
                        self.credential.clear();
                        self.validation = validate_credential("");
                        self.last_input_change = None;
                        self.error = None;
                    }
                });
            });
            ui.add_space(7.0);

            let response = credential_editor(ui, &mut self.credential, inner.width());
            if response.changed() {
                self.validation = validate_credential(&self.credential);
                self.last_input_change = (!self.credential.is_empty()).then(Instant::now);
                self.error = None;
            }
            ui.add_space(6.0);
            ui.label(
                RichText::new("输入框固定只显示三行；查询成功后会立即从内存清除。")
                    .size(11.0)
                    .color(MUTED),
            );
            ui.add_space(18.0);

            validation_box(ui, &self.validation);
            ui.add_space(16.0);

            ui.checkbox(
                &mut self.authorized,
                RichText::new("这是我自己的账户，或我已获得账户所有者明确授权")
                    .size(12.0)
                    .color(INK),
            );
            ui.add_space(14.0);

            let can_query = self.validation.can_query && self.authorized && !self.querying;
            let label = if self.querying {
                let elapsed = self
                    .query_started
                    .map(|started| started.elapsed().as_secs())
                    .unwrap_or_default();
                format!("正在核验与查询 · {elapsed}s")
            } else {
                "开始只读查询".to_string()
            };
            let button = egui::Button::new(
                RichText::new(label)
                    .size(14.0)
                    .strong()
                    .color(Color32::WHITE),
            )
            .fill(if can_query {
                INK
            } else {
                Color32::from_rgb(164, 161, 153)
            })
            .corner_radius(10.0)
            .min_size(vec2(190.0, 42.0));
            if ui.add_enabled(can_query, button).clicked() {
                self.start_query(ui.ctx());
            }

            if let Some(error) = self.error.as_deref() {
                ui.add_space(12.0);
                error_banner(ui, error);
            }
        });
    }

    fn draw_result(&mut self, ui: &mut egui::Ui) {
        let Some(result) = self.result.as_ref() else {
            return;
        };
        let tier = PlanTier::from_plan(&result.subscription.plan);
        let palette = tier.palette();
        let mut reset_requested = false;

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("账户与订阅").size(21.0).strong().color(INK));
                ui.label(
                    RichText::new(format!(
                        "查询于 {} · 结果仅保留在当前窗口",
                        format_datetime(Some(&result.queried_at))
                    ))
                    .size(11.0)
                    .color(MUTED),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("查询另一个账户").size(12.0).color(INK))
                            .fill(Color32::from_rgb(252, 251, 248))
                            .stroke(Stroke::new(1.0, BORDER))
                            .corner_radius(9.0)
                            .min_size(vec2(126.0, 34.0)),
                    )
                    .clicked()
                {
                    reset_requested = true;
                }
                ui.add_space(8.0);
                let status_text = if result.subscription.active {
                    "● 订阅有效"
                } else {
                    "● 当前未生效"
                };
                ui.label(RichText::new(status_text).size(12.0).color(
                    if result.subscription.active {
                        SUCCESS
                    } else {
                        MUTED
                    },
                ));
            });
        });
        ui.add_space(12.0);

        draw_card_front(ui, result, palette, &self.textures);
        ui.add_space(12.0);
        draw_card_back(ui, result, palette);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("未返回的额度会明确标注，不按套餐名称推算。")
                    .size(10.5)
                    .color(MUTED),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new("只读 · 无购买、取消或续费操作")
                        .size(10.5)
                        .color(MUTED),
                );
            });
        });

        if reset_requested {
            self.reset();
        }
    }
}

impl Drop for SubscriptionLensApp {
    fn drop(&mut self) {
        self.credential.zeroize();
    }
}

impl eframe::App for SubscriptionLensApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if !self.querying
            && self
                .last_input_change
                .is_some_and(|changed| changed.elapsed() >= Duration::from_secs(300))
        {
            self.credential.zeroize();
            self.credential.clear();
            self.validation = validate_credential("");
            self.last_input_change = None;
            self.error = Some("会话数据已因闲置 5 分钟自动清除。".into());
        }
        self.poll_query();
        if self.querying {
            ui.ctx().request_repaint_after(Duration::from_millis(250));
        }

        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, PAGE_BG);
        let sidebar =
            Rect::from_min_max(full.min, pos2(full.left() + SIDEBAR_WIDTH, full.bottom()));
        let main = Rect::from_min_max(
            pos2(sidebar.right() + 24.0, full.top() + 20.0),
            pos2(full.right() - 24.0, full.bottom() - 12.0),
        );
        ui.scope_builder(UiBuilder::new().max_rect(sidebar), |ui| {
            self.draw_sidebar(ui);
        });
        ui.scope_builder(UiBuilder::new().max_rect(main), |ui| {
            ui.set_width(main.width());
            if self.result.is_some() {
                self.draw_result(ui);
            } else {
                self.draw_query(ui);
            }
        });
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        PAGE_BG.to_normalized_gamma_f32()
    }
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "noto-sc".to_owned(),
        FontData::from_static(include_bytes!("../../assets/fonts/NotoSansSC-Regular.otf")).into(),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "noto-sc".to_owned());
    }
    ctx.set_fonts(fonts);
}

fn configure_style(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = PAGE_BG;
    visuals.window_fill = PAGE_BG;
    visuals.extreme_bg_color = Color32::from_rgb(250, 249, 246);
    visuals.faint_bg_color = Color32::from_rgb(239, 237, 230);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(251, 250, 247);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(246, 242, 236);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(195, 188, 177));
    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(198, 91, 63, 42);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style_of(egui::Theme::Light)).clone();
    style.spacing.item_spacing = vec2(8.0, 8.0);
    style.spacing.button_padding = vec2(12.0, 8.0);
    style.visuals.widgets.noninteractive.corner_radius = CornerRadius::same(8);
    style.visuals.widgets.inactive.corner_radius = CornerRadius::same(8);
    style.visuals.widgets.hovered.corner_radius = CornerRadius::same(8);
    style.visuals.widgets.active.corner_radius = CornerRadius::same(8);
    ctx.set_style_of(egui::Theme::Light, style);
}

fn sidebar_item(ui: &mut egui::Ui, number: &str, label: &str, active: bool) {
    let width = SIDEBAR_WIDTH - 24.0;
    let (rect, _) = ui.allocate_exact_size(vec2(width, 38.0), Sense::hover());
    if active {
        ui.painter()
            .rect_filled(rect, 10.0, Color32::from_rgba_unmultiplied(198, 91, 63, 22));
        ui.painter().rect_filled(
            Rect::from_min_size(rect.min, vec2(3.0, rect.height())),
            2.0,
            ACCENT,
        );
    }
    ui.painter().text(
        pos2(rect.left() + 15.0, rect.center().y),
        Align2::LEFT_CENTER,
        number,
        FontId::monospace(10.0),
        if active { ACCENT } else { MUTED },
    );
    ui.painter().text(
        pos2(rect.left() + 48.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(12.5),
        if active { INK } else { MUTED },
    );
}

fn header_row(ui: &mut egui::Ui, title: &str, note: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).size(22.0).strong().color(INK));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(note).size(11.0).color(MUTED));
        });
    });
}

/// Draw the credential editor inside a hard, three-line viewport.
///
/// `TextEdit::desired_rows` is only a minimum height for multiline editors. A long pasted
/// credential can therefore make the raw widget thousands of pixels tall. Reserving the outer
/// rectangle ourselves keeps every following control in place, while the nested scroll area
/// clips interaction and painting to the editor viewport.
fn credential_editor(ui: &mut egui::Ui, credential: &mut String, width: f32) -> egui::Response {
    let (frame_rect, _) =
        ui.allocate_exact_size(vec2(width, CREDENTIAL_EDITOR_HEIGHT), Sense::hover());
    ui.painter().rect(
        frame_rect,
        CornerRadius::same(8),
        Color32::from_rgb(251, 250, 247),
        Stroke::new(1.0, BORDER),
        StrokeKind::Inside,
    );

    let viewport = frame_rect.shrink2(vec2(14.0, 10.0));
    let mut editor_ui = ui.new_child(
        UiBuilder::new()
            .id_salt("credential-editor-viewport")
            .max_rect(viewport)
            .layout(Layout::top_down(Align::Min)),
    );
    editor_ui.set_clip_rect(viewport);

    egui::ScrollArea::vertical()
        .id_salt("credential-editor-scroll")
        .max_height(viewport.height())
        .min_scrolled_height(viewport.height())
        .auto_shrink([false, false])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(&mut editor_ui, |ui| {
            ui.set_min_width(viewport.width());
            ui.set_max_width(viewport.width());
            ui.add(
                egui::TextEdit::multiline(credential)
                    .id_salt("credential-editor")
                    .desired_width(viewport.width())
                    .desired_rows(3)
                    .hint_text("在这里粘贴完整内容…")
                    .font(FontId::monospace(12.0))
                    .text_color(INK)
                    .frame(egui::Frame::NONE)
                    .margin(Margin::ZERO),
            )
        })
        .inner
}

fn validation_box(ui: &mut egui::Ui, validation: &CredentialValidation) {
    let fill = if validation.valid {
        Color32::from_rgb(238, 247, 240)
    } else if validation.can_query {
        Color32::from_rgb(247, 243, 232)
    } else {
        Color32::from_rgb(247, 241, 237)
    };
    let border = if validation.valid {
        Color32::from_rgb(180, 218, 193)
    } else if validation.can_query {
        Color32::from_rgb(224, 208, 168)
    } else {
        Color32::from_rgb(226, 199, 188)
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(12.0)
        .inner_margin(Margin::symmetric(16, 13))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let icon = if validation.valid || validation.can_query {
                    "✓"
                } else {
                    "!"
                };
                ui.label(RichText::new(icon).size(15.0).strong().color(
                    if validation.valid || validation.can_query {
                        SUCCESS
                    } else {
                        DANGER
                    },
                ));
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&validation.summary)
                            .size(12.5)
                            .strong()
                            .color(INK),
                    );
                    if !validation.missing.is_empty() {
                        ui.label(
                            RichText::new(format!("仍缺少：{}", validation.missing.join("、")))
                                .size(11.0)
                                .color(MUTED),
                        );
                    } else if validation.can_query && !validation.complete {
                        ui.label(
                            RichText::new(
                                "查询时会先在线核验邮箱、账户 ID 与有效期；任一缺失即停止。",
                            )
                            .size(11.0)
                            .color(MUTED),
                        );
                    }
                });
            });
        });
}

fn error_banner(ui: &mut egui::Ui, error: &str) {
    egui::Frame::new()
        .fill(Color32::from_rgb(250, 238, 234))
        .stroke(Stroke::new(1.0, Color32::from_rgb(229, 191, 179)))
        .corner_radius(10.0)
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.label(RichText::new(error).size(11.5).color(DANGER));
        });
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PlanTier {
    Pro20x,
    Pro5x,
    Plus,
    Free,
    Other,
}

impl PlanTier {
    fn from_plan(plan: &str) -> Self {
        let normalized = plan.to_ascii_lowercase().replace([' ', '-', '_'], "");
        if normalized.contains("pro20x") || normalized == "pro" {
            Self::Pro20x
        } else if normalized.contains("pro5x") || normalized.contains("prolite") {
            Self::Pro5x
        } else if normalized.contains("plus") {
            Self::Plus
        } else if normalized.contains("free") || normalized.is_empty() {
            Self::Free
        } else {
            Self::Other
        }
    }

    fn palette(self) -> PlanPalette {
        match self {
            Self::Pro20x => PlanPalette {
                base: Color32::from_rgb(22, 21, 18),
                raised: Color32::from_rgb(35, 32, 25),
                accent: Color32::from_rgb(218, 185, 111),
                soft: Color32::from_rgb(117, 91, 42),
            },
            Self::Pro5x => PlanPalette {
                base: Color32::from_rgb(18, 48, 92),
                raised: Color32::from_rgb(24, 62, 116),
                accent: Color32::from_rgb(123, 175, 255),
                soft: Color32::from_rgb(45, 91, 158),
            },
            Self::Plus => PlanPalette {
                base: Color32::from_rgb(20, 61, 45),
                raised: Color32::from_rgb(28, 78, 58),
                accent: Color32::from_rgb(116, 216, 159),
                soft: Color32::from_rgb(48, 117, 83),
            },
            Self::Free => PlanPalette {
                base: Color32::from_rgb(72, 74, 76),
                raised: Color32::from_rgb(88, 91, 94),
                accent: Color32::from_rgb(213, 216, 218),
                soft: Color32::from_rgb(113, 117, 121),
            },
            Self::Other => PlanPalette {
                base: Color32::from_rgb(55, 47, 57),
                raised: Color32::from_rgb(73, 62, 76),
                accent: Color32::from_rgb(220, 180, 214),
                soft: Color32::from_rgb(111, 83, 108),
            },
        }
    }
}

#[derive(Clone, Copy)]
struct PlanPalette {
    base: Color32,
    raised: Color32,
    accent: Color32,
    soft: Color32,
}

fn draw_card_front(
    ui: &mut egui::Ui,
    result: &InspectionResult,
    palette: PlanPalette,
    textures: &PaymentTextures,
) {
    let size = vec2(ui.available_width(), 248.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 24.0, palette.base);
    painter.circle_filled(
        pos2(rect.right() - 80.0, rect.top() + 10.0),
        205.0,
        palette.soft.gamma_multiply(0.22),
    );
    painter.circle_stroke(
        pos2(rect.right() - 112.0, rect.top() + 18.0),
        142.0,
        Stroke::new(1.0, palette.accent.gamma_multiply(0.18)),
    );

    let left = rect.left() + 34.0;
    painter.text(
        pos2(left, rect.top() + 28.0),
        Align2::LEFT_TOP,
        result
            .identity
            .email
            .as_deref()
            .map(|email| truncate_middle(email, 34))
            .unwrap_or_else(|| "邮箱未返回".into()),
        FontId::proportional(11.5),
        Color32::from_white_alpha(175),
    );
    painter.text(
        pos2(left, rect.top() + 60.0),
        Align2::LEFT_TOP,
        &result.subscription.plan,
        FontId::proportional(38.0),
        palette.accent,
    );
    painter.text(
        pos2(left, rect.top() + 111.0),
        Align2::LEFT_TOP,
        if result.subscription.active {
            "CURRENT PLAN · 有效"
        } else {
            "CURRENT PLAN · 未生效"
        },
        FontId::monospace(10.0),
        Color32::from_white_alpha(145),
    );

    let remaining = format_remaining(result.subscription.remaining_seconds);
    painter.text(
        pos2(left, rect.top() + 145.0),
        Align2::LEFT_TOP,
        remaining,
        FontId::proportional(23.0),
        Color32::WHITE,
    );
    painter.text(
        pos2(left, rect.top() + 181.0),
        Align2::LEFT_TOP,
        format!(
            "到期 {}   ·   自动续费 {}",
            format_datetime(result.subscription.active_until.as_deref()),
            yes_no_unknown(result.subscription.will_renew)
        ),
        FontId::proportional(11.5),
        Color32::from_white_alpha(170),
    );

    let payment_rect = Rect::from_min_max(
        pos2(rect.right() - 404.0, rect.top() + 24.0),
        pos2(rect.right() - 24.0, rect.bottom() - 24.0),
    );
    painter.rect(
        payment_rect,
        CornerRadius::same(18),
        Color32::from_rgb(18, 19, 20),
        Stroke::new(1.0, Color32::from_white_alpha(32)),
        StrokeKind::Inside,
    );
    painter.rect_filled(
        Rect::from_min_size(
            pos2(payment_rect.left() + 22.0, payment_rect.top() + 22.0),
            vec2(38.0, 28.0),
        ),
        6.0,
        Color32::from_rgb(186, 160, 89),
    );
    painter.text(
        pos2(payment_rect.left() + 22.0, payment_rect.top() + 70.0),
        Align2::LEFT_TOP,
        payment_channel_label(result),
        FontId::proportional(12.0),
        Color32::from_white_alpha(135),
    );
    painter.text(
        pos2(payment_rect.left() + 22.0, payment_rect.top() + 95.0),
        Align2::LEFT_TOP,
        payment_number(result),
        FontId::monospace(15.0),
        Color32::WHITE,
    );
    painter.text(
        pos2(payment_rect.left() + 22.0, payment_rect.bottom() - 24.0),
        Align2::LEFT_BOTTOM,
        payment_meta(result),
        FontId::proportional(10.5),
        Color32::from_white_alpha(120),
    );

    if let Some(texture) = payment_texture(result, textures) {
        let image_rect = Rect::from_center_size(
            pos2(payment_rect.right() - 88.0, payment_rect.center().y - 2.0),
            vec2(140.0, 140.0),
        );
        painter.image(
            texture.id(),
            image_rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        painter.text(
            pos2(payment_rect.right() - 84.0, payment_rect.center().y),
            Align2::CENTER_CENTER,
            "◈",
            FontId::proportional(58.0),
            Color32::from_white_alpha(90),
        );
    }
}

fn draw_card_back(ui: &mut egui::Ui, result: &InspectionResult, palette: PlanPalette) {
    let size = vec2(ui.available_width(), 382.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 24.0, palette.raised);
    painter.rect_stroke(
        rect.shrink(0.5),
        24.0,
        Stroke::new(1.0, Color32::from_white_alpha(24)),
        StrokeKind::Inside,
    );

    let divider_x = rect.left() + rect.width() * 0.63;
    painter.line_segment(
        [
            pos2(divider_x, rect.top() + 28.0),
            pos2(divider_x, rect.bottom() - 28.0),
        ],
        Stroke::new(1.0, Color32::from_white_alpha(28)),
    );

    let left = rect.left() + 30.0;
    let top = rect.top() + 25.0;
    painter.text(
        pos2(left, top),
        Align2::LEFT_TOP,
        "账户与额度",
        FontId::proportional(17.0),
        Color32::WHITE,
    );
    painter.text(
        pos2(left + 132.0, top + 2.0),
        Align2::LEFT_TOP,
        account_summary(result),
        FontId::proportional(10.5),
        Color32::from_white_alpha(135),
    );

    let slots = quota_slots(&result.usage);
    for (index, slot) in slots.iter().enumerate() {
        let y = top + 44.0 + index as f32 * 50.0;
        draw_quota_row(
            painter,
            left,
            divider_x - left - 30.0,
            y,
            slot,
            palette.accent,
        );
    }

    let detail_y = rect.bottom() - 41.0;
    painter.text(
        pos2(left, detail_y),
        Align2::LEFT_CENTER,
        format!(
            "币种 {}   ·   周期 {}   ·   购买渠道 {}",
            result.subscription.currency.as_deref().unwrap_or("未返回"),
            billing_period_label(result.subscription.billing_period.as_deref()),
            store_label(result.subscription.purchase_origin.as_deref())
        ),
        FontId::proportional(10.5),
        Color32::from_white_alpha(135),
    );

    let right = divider_x + 26.0;
    painter.text(
        pos2(right, top),
        Align2::LEFT_TOP,
        "账单记录",
        FontId::proportional(17.0),
        Color32::WHITE,
    );
    painter.text(
        pos2(rect.right() - 28.0, top + 3.0),
        Align2::RIGHT_TOP,
        "最多显示最近 4 条",
        FontId::proportional(10.0),
        Color32::from_white_alpha(110),
    );

    let records = combined_records(result);
    if records.is_empty() {
        painter.text(
            pos2(right, top + 58.0),
            Align2::LEFT_TOP,
            "未返回可确认的账单记录",
            FontId::proportional(12.0),
            Color32::from_white_alpha(135),
        );
        painter.text(
            pos2(right, top + 82.0),
            Align2::LEFT_TOP,
            "Apple / Google 完整收据请在原商店核对",
            FontId::proportional(10.0),
            Color32::from_white_alpha(90),
        );
    } else {
        for (index, record) in records.iter().take(4).enumerate() {
            let y = top + 43.0 + index as f32 * 71.0;
            draw_billing_row(painter, right, rect.right() - right - 28.0, y, record);
        }
    }
}

fn draw_quota_row(
    painter: &egui::Painter,
    x: f32,
    width: f32,
    y: f32,
    slot: &QuotaSlot,
    accent: Color32,
) {
    painter.text(
        pos2(x, y),
        Align2::LEFT_TOP,
        slot.label,
        FontId::proportional(11.5),
        Color32::from_white_alpha(205),
    );
    painter.text(
        pos2(x + width, y),
        Align2::RIGHT_TOP,
        &slot.value,
        FontId::proportional(11.5),
        if slot.remaining.is_some() {
            accent
        } else {
            Color32::from_white_alpha(105)
        },
    );
    let bar = Rect::from_min_size(pos2(x, y + 25.0), vec2(width, 5.0));
    painter.rect_filled(bar, 3.0, Color32::from_white_alpha(26));
    if let Some(remaining) = slot.remaining {
        let fill = Rect::from_min_size(
            bar.min,
            vec2(bar.width() * (remaining as f32 / 100.0), bar.height()),
        );
        painter.rect_filled(fill, 3.0, accent);
    }
    painter.text(
        pos2(x, y + 36.0),
        Align2::LEFT_TOP,
        &slot.note,
        FontId::proportional(9.5),
        Color32::from_white_alpha(90),
    );
}

fn draw_billing_row(painter: &egui::Painter, x: f32, width: f32, y: f32, record: &BillingRecord) {
    let icon_rect = Rect::from_min_size(pos2(x, y + 3.0), vec2(31.0, 31.0));
    painter.rect_filled(icon_rect, 9.0, Color32::from_white_alpha(18));
    painter.text(
        icon_rect.center(),
        Align2::CENTER_CENTER,
        billing_icon(record.store.as_deref()),
        FontId::proportional(13.0),
        Color32::from_white_alpha(190),
    );
    painter.text(
        pos2(x + 43.0, y),
        Align2::LEFT_TOP,
        truncate_end(&record.product, 22),
        FontId::proportional(11.5),
        Color32::WHITE,
    );
    painter.text(
        pos2(x + width, y),
        Align2::RIGHT_TOP,
        record_amount(record),
        FontId::monospace(10.5),
        Color32::from_white_alpha(205),
    );
    painter.text(
        pos2(x + 43.0, y + 23.0),
        Align2::LEFT_TOP,
        format!(
            "{} · {}",
            format_datetime(record.created_at.as_deref()),
            status_label(&record.status)
        ),
        FontId::proportional(9.5),
        Color32::from_white_alpha(105),
    );
    painter.line_segment(
        [pos2(x + 43.0, y + 48.0), pos2(x + width, y + 48.0)],
        Stroke::new(1.0, Color32::from_white_alpha(18)),
    );
}

struct QuotaSlot {
    label: &'static str,
    value: String,
    note: String,
    remaining: Option<f64>,
}

fn quota_slots(usage: &UsageSummary) -> Vec<QuotaSlot> {
    let mut assigned = vec![false; usage.windows.len()];
    let image = find_window(
        &usage.windows,
        &mut assigned,
        &["image", "dall", "sora", "生图"],
    );
    let research = find_window(
        &usage.windows,
        &mut assigned,
        &["deepresearch", "deep research", "research", "深度"],
    );
    let chat = find_window(&usage.windows, &mut assigned, &["chat", "message", "对话"]);
    let pro = find_window(&usage.windows, &mut assigned, &["pro", "网页"]);
    let codex = usage
        .windows
        .iter()
        .enumerate()
        .find(|(index, _)| !assigned[*index])
        .map(|(index, window)| {
            assigned[index] = true;
            window
        });

    [
        ("Codex 额度", codex),
        ("Chat 额度", chat),
        ("网页端 Pro 额度", pro),
        ("生图额度", image),
        ("Deep Research 额度", research),
    ]
    .into_iter()
    .map(|(label, window)| quota_slot(label, window, usage))
    .collect()
}

fn find_window<'a>(
    windows: &'a [UsageWindow],
    assigned: &mut [bool],
    needles: &[&str],
) -> Option<&'a UsageWindow> {
    for (index, window) in windows.iter().enumerate() {
        if assigned[index] {
            continue;
        }
        let haystack = format!(
            "{} {}",
            window.label.to_ascii_lowercase(),
            window.model.as_deref().unwrap_or("").to_ascii_lowercase()
        );
        if needles.iter().any(|needle| haystack.contains(needle)) {
            assigned[index] = true;
            return Some(window);
        }
    }
    None
}

fn quota_slot(
    label: &'static str,
    window: Option<&UsageWindow>,
    usage: &UsageSummary,
) -> QuotaSlot {
    if let Some(window) = window {
        QuotaSlot {
            label,
            value: format!("剩余 {:.0}%", window.remaining_percent),
            note: window
                .reset_at
                .as_deref()
                .map(|value| format!("恢复 {}", format_datetime(Some(value))))
                .or_else(|| {
                    window
                        .reset_after_seconds
                        .map(|seconds| format!("约 {} 后恢复", format_duration(seconds)))
                })
                .unwrap_or_else(|| window.label.clone()),
            remaining: Some(window.remaining_percent),
        }
    } else {
        QuotaSlot {
            label,
            value: "未返回".into(),
            note: if usage.status == "unavailable" {
                "额度端点当前不可用".into()
            } else {
                "本次响应没有可验证字段".into()
            },
            remaining: None,
        }
    }
}

fn payment_texture<'a>(
    result: &InspectionResult,
    textures: &'a PaymentTextures,
) -> Option<&'a TextureHandle> {
    let origin = result
        .subscription
        .purchase_origin
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    if origin.contains("ios") || origin.contains("apple") || origin.contains("app_store") {
        return Some(&textures.app_store);
    }
    if origin.contains("android") || origin.contains("google") || origin.contains("play_store") {
        return Some(&textures.google_play);
    }
    let brand = preferred_payment_method(result).map(|method| method.brand.as_str());
    match brand {
        Some("visa") => Some(&textures.visa),
        Some("mastercard") => Some(&textures.mastercard),
        _ => None,
    }
}

fn preferred_payment_method(result: &InspectionResult) -> Option<&PaymentMethodSummary> {
    result
        .payment_methods
        .iter()
        .find(|method| method.is_default)
        .or_else(|| result.payment_methods.first())
}

fn payment_channel_label(result: &InspectionResult) -> String {
    let origin = result
        .subscription
        .purchase_origin
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    if origin.contains("ios") || origin.contains("apple") || origin.contains("app_store") {
        "APPLE APP STORE".into()
    } else if origin.contains("android")
        || origin.contains("google")
        || origin.contains("play_store")
    {
        "GOOGLE PLAY".into()
    } else {
        preferred_payment_method(result)
            .map(|method| match method.brand.as_str() {
                "visa" => "VISA CARD",
                "mastercard" => "MASTERCARD",
                _ => "BANK CARD",
            })
            .unwrap_or("PAYMENT METHOD")
            .into()
    }
}

fn payment_number(result: &InspectionResult) -> String {
    let origin = result
        .subscription
        .purchase_origin
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    if origin.contains("ios") || origin.contains("apple") || origin.contains("app_store") {
        return "由 Apple 管理".into();
    }
    if origin.contains("android") || origin.contains("google") || origin.contains("play_store") {
        return "由 Google Play 管理".into();
    }
    preferred_payment_method(result)
        .map(masked_card_number)
        .unwrap_or_else(|| "支付信息未返回".into())
}

fn masked_card_number(method: &PaymentMethodSummary) -> String {
    match (method.first6.as_deref(), method.last4.as_deref()) {
        (Some(first6), Some(last4)) => format!("{first6}  ••••  {last4}"),
        (None, Some(last4)) => format!("••••  ••••  ••••  {last4}"),
        (Some(first6), None) => format!("{first6}  ••••  ••••"),
        (None, None) => "仅返回卡品牌".into(),
    }
}

fn payment_meta(result: &InspectionResult) -> String {
    if let Some(method) = preferred_payment_method(result) {
        if let (Some(month), Some(year)) = (method.exp_month, method.exp_year) {
            return format!("有效期 {month:02}/{:02}", year % 100);
        }
    }
    let amount = result
        .subscription
        .current_amount
        .map(|amount| format!("{amount:.2}"))
        .unwrap_or_else(|| "金额未返回".into());
    let currency = result.subscription.currency.as_deref().unwrap_or("");
    format!("{amount} {currency}").trim().to_string()
}

fn account_summary(result: &InspectionResult) -> String {
    let email = result
        .identity
        .email
        .as_deref()
        .map(|value| truncate_middle(value, 28))
        .unwrap_or_else(|| "邮箱未返回".into());
    format!(
        "{email} · {}",
        subscription_state_label(&result.subscription.state)
    )
}

fn combined_records(result: &InspectionResult) -> Vec<&BillingRecord> {
    let mut records = result
        .invoices
        .iter()
        .chain(result.mobile_records.iter())
        .collect::<Vec<_>>();
    records.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    records
}

fn record_amount(record: &BillingRecord) -> String {
    match (record.amount, record.currency.as_deref()) {
        (Some(amount), Some(currency)) => format!("{amount:.2} {currency}"),
        (Some(amount), None) => format!("{amount:.2}"),
        _ => "金额未返回".into(),
    }
}

fn format_datetime(value: Option<&str>) -> String {
    let Some(value) = value else {
        return "未返回".into();
    };
    DateTime::parse_from_rfc3339(value)
        .map(|time| {
            time.with_timezone(&Local)
                .format("%Y/%m/%d %H:%M")
                .to_string()
        })
        .unwrap_or_else(|_| truncate_end(value, 20))
}

fn format_remaining(seconds: Option<i64>) -> String {
    match seconds {
        Some(seconds) if seconds > 0 => format!("剩余 {}", format_duration(seconds)),
        Some(_) => "已到期".into(),
        None => "剩余时间未返回".into(),
    }
}

fn format_duration(seconds: i64) -> String {
    let seconds = seconds.max(0);
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    if days > 0 {
        format!("{days} 天 {hours} 小时")
    } else {
        let minutes = (seconds % 3_600) / 60;
        format!("{hours} 小时 {minutes} 分钟")
    }
}

fn yes_no_unknown(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "已开启",
        Some(false) => "未开启",
        None => "未返回",
    }
}

fn store_label(value: Option<&str>) -> &'static str {
    let value = value.unwrap_or("").to_ascii_lowercase();
    if value.contains("ios") || value.contains("apple") || value.contains("app_store") {
        "App Store"
    } else if value.contains("android") || value.contains("google") || value.contains("play_store")
    {
        "Google Play"
    } else if value.is_empty() {
        "未返回"
    } else {
        "ChatGPT 网页"
    }
}

fn billing_period_label(value: Option<&str>) -> &'static str {
    match value.unwrap_or("").to_ascii_lowercase().as_str() {
        "month" | "monthly" => "按月",
        "year" | "yearly" | "annual" => "按年",
        "" => "未返回",
        _ => "其他",
    }
}

fn subscription_state_label(value: &str) -> &'static str {
    match value {
        "active" => "订阅有效",
        "delinquent" => "待处理",
        "expired" => "已过期",
        "free" => "免费账户",
        _ => "状态未返回",
    }
}

fn status_label(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "paid" | "succeeded" | "success" | "active" => "已支付",
        "refunded" => "已退款",
        "failed" | "uncollectible" => "支付失败",
        "void" | "cancelled" | "canceled" => "已取消",
        _ => "状态未返回",
    }
}

fn billing_icon(store: Option<&str>) -> &'static str {
    let store = store.unwrap_or("").to_ascii_lowercase();
    if store.contains("apple") || store.contains("ios") {
        "A"
    } else if store.contains("google") || store.contains("play") {
        "G"
    } else {
        "¥"
    }
}

fn truncate_middle(value: &str, max_chars: usize) -> String {
    let characters = value.chars().collect::<Vec<_>>();
    if characters.len() <= max_chars || max_chars < 5 {
        return value.to_string();
    }
    let left = (max_chars - 1) / 2;
    let right = max_chars - 1 - left;
    format!(
        "{}…{}",
        characters[..left].iter().collect::<String>(),
        characters[characters.len() - right..]
            .iter()
            .collect::<String>()
    )
}

fn truncate_end(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let take = max_chars.saturating_sub(1);
    format!("{}…", value.chars().take(take).collect::<String>())
}

fn demo_result() -> InspectionResult {
    InspectionResult {
        queried_at: Local::now().to_rfc3339(),
        identity: AccountIdentity {
            name: Some("Demo User".into()),
            email: Some("account@example.com".into()),
            user_id: Some("user-demo".into()),
            account_id: Some("account-demo".into()),
            account_created_at: None,
            token_expires_at: None,
            claimed_plan: Some("pro".into()),
            has_previously_paid: Some(true),
            deactivated: Some(false),
        },
        subscription: SubscriptionSummary {
            state: "active".into(),
            plan: "Pro 20X".into(),
            plan_code: Some("pro".into()),
            active: true,
            active_start: Some(Local::now().to_rfc3339()),
            active_until: Some((Local::now() + chrono::Duration::days(18)).to_rfc3339()),
            remaining_seconds: Some(18 * 86_400 + 11 * 3_600),
            will_renew: Some(true),
            purchase_origin: Some("chatgpt_web".into()),
            billing_period: Some("month".into()),
            currency: Some("USD".into()),
            current_amount: Some(200.0),
            current_amount_source: None,
            delinquent: false,
            grace_period_end: None,
            free_reason: None,
            seats_in_use: None,
            seats_entitled: None,
        },
        usage: UsageSummary {
            status: "available".into(),
            plan_type: Some("pro".into()),
            windows: vec![
                UsageWindow {
                    label: "5 小时滚动窗口".into(),
                    used_percent: 31.0,
                    remaining_percent: 69.0,
                    reset_at: Some((Local::now() + chrono::Duration::hours(2)).to_rfc3339()),
                    reset_after_seconds: Some(7_200),
                    window_seconds: Some(18_000),
                    model: Some("codex".into()),
                },
                UsageWindow {
                    label: "image generation".into(),
                    used_percent: 58.0,
                    remaining_percent: 42.0,
                    reset_at: None,
                    reset_after_seconds: Some(86_400),
                    window_seconds: Some(86_400),
                    model: Some("image_generation".into()),
                },
                UsageWindow {
                    label: "deep research".into(),
                    used_percent: 80.0,
                    remaining_percent: 20.0,
                    reset_at: None,
                    reset_after_seconds: Some(259_200),
                    window_seconds: Some(604_800),
                    model: Some("deep_research".into()),
                },
            ],
            credits: None,
            limit_reached: Some(false),
            message: None,
        },
        invoices: vec![
            demo_invoice("2026-09", "ChatGPT Pro", 200.0, "paid", 1),
            demo_invoice("2026-08", "ChatGPT Pro", 200.0, "paid", 31),
            demo_invoice("2026-07", "ChatGPT Pro", 200.0, "paid", 61),
        ],
        mobile_records: Vec::new(),
        payment_methods: vec![PaymentMethodSummary {
            kind: "card".into(),
            brand: "mastercard".into(),
            first6: Some("555555".into()),
            last4: Some("4444".into()),
            exp_month: Some(12),
            exp_year: Some(2029),
            is_default: true,
            source_note: String::new(),
        }],
        sources: Vec::new(),
        warnings: Vec::new(),
        coverage: CoverageInfo::default(),
    }
}

fn demo_invoice(
    id: &str,
    product: &str,
    amount: f64,
    status: &str,
    days_ago: i64,
) -> BillingRecord {
    BillingRecord {
        id: id.into(),
        record_type: "web_invoice".into(),
        product: product.into(),
        status: status.into(),
        created_at: Some((Local::now() - chrono::Duration::days(days_ago)).to_rfc3339()),
        period_start: None,
        period_end: None,
        amount: Some(amount),
        currency: Some("USD".into()),
        store: Some("chatgpt_web".into()),
        will_renew: None,
        refunded_at: None,
        source_note: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_required_plan_colors_to_distinct_tiers() {
        assert_eq!(PlanTier::from_plan("Pro 20X"), PlanTier::Pro20x);
        assert_eq!(PlanTier::from_plan("pro_5x"), PlanTier::Pro5x);
        assert_eq!(PlanTier::from_plan("Plus"), PlanTier::Plus);
        assert_eq!(PlanTier::from_plan("Free"), PlanTier::Free);
        assert_ne!(
            PlanTier::Pro20x.palette().base,
            PlanTier::Pro5x.palette().base
        );
        assert_ne!(PlanTier::Plus.palette().base, PlanTier::Free.palette().base);
    }

    #[test]
    fn card_number_never_reconstructs_more_than_first_six_and_last_four() {
        let method = PaymentMethodSummary {
            first6: Some("424242".into()),
            last4: Some("4242".into()),
            ..Default::default()
        };
        let display = masked_card_number(&method);
        assert_eq!(display, "424242  ••••  4242");
        assert!(!display.contains("4242424242424242"));
    }

    #[test]
    fn quota_slots_keep_unknown_values_explicit() {
        let usage = UsageSummary {
            status: "available".into(),
            ..Default::default()
        };
        let slots = quota_slots(&usage);
        assert_eq!(slots.len(), 5);
        assert!(slots.iter().all(|slot| slot.value == "未返回"));
    }

    #[test]
    fn truncation_is_unicode_safe() {
        assert_eq!(truncate_end("订阅镜桌面版", 5), "订阅镜桌…");
        assert_eq!(truncate_middle("account@example.com", 10), "acco…e.com");
    }

    #[test]
    fn long_credential_is_clipped_and_cannot_cover_the_query_controls() {
        let ctx = egui::Context::default();
        configure_fonts(&ctx);
        configure_style(&ctx);
        let raw_input = |events| egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1_000.0, 800.0))),
            events,
            ..Default::default()
        };
        let mut credential = format!(
            "{{\"accessToken\":\"{}\",\"account\":{{\"id\":\"account-test\"}}}}",
            "synthetic-token".repeat(4_000)
        );
        let mut editor_top = 0.0;
        let mut editor_interact_rect = Rect::ZERO;
        let mut button_rect = Rect::ZERO;

        for _ in 0..2 {
            let _ = ctx.run_ui(raw_input(Vec::new()), |ui| {
                ui.set_width(900.0);
                editor_top = ui.next_widget_position().y;
                let response = credential_editor(ui, &mut credential, 900.0);
                let button = ui.button("开始只读查询");
                editor_interact_rect = response.interact_rect;
                button_rect = button.rect;
            });
        }

        assert!(editor_interact_rect.height() > 0.0);
        assert!(editor_interact_rect.height() <= CREDENTIAL_EDITOR_HEIGHT);
        assert!(button_rect.top() < editor_top + CREDENTIAL_EDITOR_HEIGHT + 20.0);
        assert!(!editor_interact_rect.intersects(button_rect));

        let click_position = button_rect.center();
        let click_events = vec![
            egui::Event::PointerMoved(click_position),
            egui::Event::PointerButton {
                pos: click_position,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: click_position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ];
        let mut button_clicked = false;
        let _ = ctx.run_ui(raw_input(click_events), |ui| {
            ui.set_width(900.0);
            let _ = credential_editor(ui, &mut credential, 900.0);
            let button = ui.button("开始只读查询");
            button_clicked = button.clicked();
        });
        assert!(button_clicked);
    }
}
