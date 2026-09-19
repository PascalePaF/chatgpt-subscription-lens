import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

type AppInfo = {
  name: string;
  version: string;
  localOnly: boolean;
  readOnly: boolean;
  allowedHosts: string[];
};

type AccountIdentity = {
  name?: string;
  email?: string;
  userId?: string;
  accountId?: string;
  accountCreatedAt?: string;
  tokenExpiresAt?: string;
  claimedPlan?: string;
  hasPreviouslyPaid?: boolean;
  deactivated?: boolean;
};

type SubscriptionSummary = {
  state: string;
  plan: string;
  planCode?: string;
  active: boolean;
  activeStart?: string;
  activeUntil?: string;
  remainingSeconds?: number;
  willRenew?: boolean;
  purchaseOrigin?: string;
  billingPeriod?: string;
  currency?: string;
  currentAmount?: number;
  currentAmountSource?: string;
  delinquent: boolean;
  gracePeriodEnd?: string;
  freeReason?: string;
  seatsInUse?: number;
  seatsEntitled?: number;
};

type UsageWindow = {
  label: string;
  usedPercent: number;
  remainingPercent: number;
  resetAt?: string;
  resetAfterSeconds?: number;
  windowSeconds?: number;
  model?: string;
};

type UsageSummary = {
  status: string;
  planType?: string;
  windows: UsageWindow[];
  credits?: unknown;
  limitReached?: boolean;
  message?: string;
};

type BillingRecord = {
  id: string;
  recordType: string;
  product: string;
  status: string;
  createdAt?: string;
  periodStart?: string;
  periodEnd?: string;
  amount?: number;
  currency?: string;
  store?: string;
  willRenew?: boolean;
  refundedAt?: string;
  sourceNote?: string;
};

type SourceStatus = {
  id: string;
  label: string;
  status: string;
  detail: string;
  officialPublicApi: boolean;
};

type InspectionResult = {
  queriedAt: string;
  identity: AccountIdentity;
  subscription: SubscriptionSummary;
  usage: UsageSummary;
  invoices: BillingRecord[];
  mobileRecords: BillingRecord[];
  sources: SourceStatus[];
  warnings: string[];
  coverage: {
    currentSubscription: string;
    webInvoiceHistory: string;
    mobileHistory: string;
    quota: string;
  };
};

const appElement = document.querySelector<HTMLDivElement>("#app");
if (!appElement) throw new Error("Missing #app");
const app: HTMLDivElement = appElement;

let appInfo: AppInfo = {
  name: "订阅镜",
  version: "1.0.0",
  localOnly: true,
  readOnly: true,
  allowedHosts: ["chatgpt.com"],
};
let clearTimer: number | undefined;

app.innerHTML = landingMarkup();
bindLandingEvents();
void loadAppInfo();

function landingMarkup(): string {
  return `
    <div class="app-backdrop" aria-hidden="true">
      <span class="orb orb-a"></span><span class="orb orb-b"></span><span class="orb orb-c"></span>
    </div>
    <div class="shell">
      <header class="topbar">
        <button class="brand" type="button" data-action="home" aria-label="返回查询页">
          <span class="brand-mark"><span>镜</span></span>
          <span class="brand-copy"><strong>订阅镜</strong><small>SUBSCRIPTION LENS</small></span>
        </button>
        <div class="topbar-actions">
          <span class="local-pill"><i></i> 本地只读</span>
          <button class="icon-button" type="button" data-action="theme" aria-label="切换明暗主题" title="切换明暗主题">◐</button>
        </div>
      </header>

      <main id="main-content">
        <section class="landing-grid">
          <div class="hero-copy">
            <p class="eyebrow"><span></span> 凭证不离开你的电脑</p>
            <h1>看清订阅，<br><em>不交出会话。</em></h1>
            <p class="hero-lead">把 ChatGPT 会话数据交给本机程序，核对套餐、币种、续费、到期时间、网页账单和 Codex 额度。</p>
            <div class="feature-ribbon" aria-label="产品特性">
              <div><b>01</b><span><strong>全程本地</strong><small>没有中转服务器</small></span></div>
              <div><b>02</b><span><strong>只读查询</strong><small>不取消、不续费</small></span></div>
              <div><b>03</b><span><strong>开源可审计</strong><small>请求目标固定</small></span></div>
            </div>
          </div>

          <section class="query-card" aria-labelledby="query-title">
            <div class="card-kicker"><span>01</span><div><small>开始查询</small><h2 id="query-title">粘贴会话凭证</h2></div></div>
            <p class="query-guide">支持完整 Session JSON、以 <code>eyJ</code> 开头的 Access Token，或 Codex <code>auth.json</code> 内容。</p>
            <label class="credential-field">
              <span>会话数据</span>
              <textarea id="credential-input" rows="8" autocomplete="off" autocapitalize="off" spellcheck="false" placeholder='{ "user": { "email": "…" }, "accessToken": "eyJ…" }'></textarea>
              <span class="field-corner">仅内存</span>
            </label>
            <div class="input-actions">
              <button class="text-button" type="button" data-action="paste">从剪贴板粘贴</button>
              <button class="text-button danger-text" type="button" data-action="clear-input">清空</button>
            </div>
            <label class="consent-row">
              <input id="ownership-confirm" type="checkbox" />
              <span>我只查询自己拥有或获明确授权访问的账号</span>
            </label>
            <div id="query-error" class="inline-error" role="alert" hidden></div>
            <button id="query-button" class="primary-button" type="button" data-action="query">
              <span>开始本地核对</span><b>→</b>
            </button>
            <p class="privacy-note"><span>●</span> 查询成功后输入框立即清空；程序不写入凭证文件。</p>
          </section>
        </section>

        <section class="trust-panel">
          <div class="trust-heading"><span>安全边界</span><h2>它会做什么，也会克制什么</h2></div>
          <div class="trust-grid">
            <article><span class="trust-icon">⌁</span><h3>固定网络目标</h3><p>只连接 <code>chatgpt.com</code>，不把 Session 发给查询站或统计服务。</p></article>
            <article><span class="trust-icon">◎</span><h3>只读端点</h3><p>只读取账户、订阅、账单和额度；没有购买、取消、恢复续费代码。</p></article>
            <article><span class="trust-icon">◇</span><h3>诚实标注覆盖</h3><p>移动商店完整收据仍归 Apple/Google 管理；查不到的内容不会猜测。</p></article>
          </div>
        </section>
      </main>

      <footer class="footer"><span>订阅镜 v<span data-version>1.0.0</span></span><span>MIT 开源 · 无遥测 · 本地处理</span></footer>
    </div>`;
}

function bindLandingEvents(): void {
  app.querySelectorAll<HTMLElement>("[data-action]").forEach((element) => {
    element.addEventListener("click", () => void handleAction(element.dataset.action ?? ""));
  });
  const input = credentialInput();
  input?.addEventListener("input", scheduleInputClear);
  input?.addEventListener("keydown", (event) => {
    if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
      event.preventDefault();
      void runInspection();
    }
  });
}

async function handleAction(action: string): Promise<void> {
  if (action === "theme") toggleTheme();
  if (action === "paste") await pasteCredential();
  if (action === "clear-input") clearCredential();
  if (action === "query") await runInspection();
  if (action === "home" || action === "new-query") showLanding();
  if (action === "clear-result") clearResult();
}

async function loadAppInfo(): Promise<void> {
  try {
    appInfo = await invoke<AppInfo>("app_info");
  } catch {
    // Browser preview keeps the built-in metadata; desktop calls replace it.
  }
  updateVersionLabels();
}

function updateVersionLabels(): void {
  document.querySelectorAll<HTMLElement>("[data-version]").forEach((node) => {
    node.textContent = appInfo.version;
  });
}

function credentialInput(): HTMLTextAreaElement | null {
  return app.querySelector<HTMLTextAreaElement>("#credential-input");
}

function scheduleInputClear(): void {
  if (clearTimer !== undefined) window.clearTimeout(clearTimer);
  clearTimer = window.setTimeout(() => {
    clearCredential();
    showInputError("为保护凭证，闲置 5 分钟后输入已自动清空。", false);
  }, 5 * 60 * 1000);
}

function clearCredential(): void {
  const input = credentialInput();
  if (input) input.value = "";
  if (clearTimer !== undefined) {
    window.clearTimeout(clearTimer);
    clearTimer = undefined;
  }
}

async function pasteCredential(): Promise<void> {
  try {
    const value = await navigator.clipboard.readText();
    const input = credentialInput();
    if (input) {
      input.value = value.trim();
      input.focus();
      scheduleInputClear();
      showInputError("", false);
    }
  } catch {
    showInputError("无法读取剪贴板，请使用 Ctrl+V 粘贴。", true);
  }
}

async function runInspection(): Promise<void> {
  const input = credentialInput();
  const consent = app.querySelector<HTMLInputElement>("#ownership-confirm");
  const button = app.querySelector<HTMLButtonElement>("#query-button");
  const value = input?.value.trim() ?? "";
  if (!value) {
    showInputError("请先粘贴 Session JSON、Access Token 或 Codex auth.json。", true);
    input?.focus();
    return;
  }
  if (!consent?.checked) {
    showInputError("请确认你有权访问该账号。", true);
    consent?.focus();
    return;
  }

  showInputError("", false);
  setQueryBusy(button, true);
  showProgress();
  try {
    const result = await invoke<InspectionResult>("inspect_subscription", {
      request: {
        credential: value,
        timezoneOffsetMin: -new Date().getTimezoneOffset(),
      },
    });
    clearCredential();
    if (input) input.value = "";
    renderDashboard(result);
  } catch (error) {
    hideProgress();
    showInputError(normalizeInvokeError(error), true);
  } finally {
    setQueryBusy(button, false);
  }
}

function setQueryBusy(button: HTMLButtonElement | null, busy: boolean): void {
  if (!button) return;
  button.disabled = busy;
  button.innerHTML = busy
    ? '<span class="button-loading"><i></i> 正在核对订阅</span><b>···</b>'
    : "<span>开始本地核对</span><b>→</b>";
}

function showProgress(): void {
  let panel = app.querySelector<HTMLDivElement>("#query-progress");
  if (!panel) {
    panel = document.createElement("div");
    panel.id = "query-progress";
    panel.className = "query-progress";
    panel.innerHTML = `
      <div class="progress-orbit"><i></i><span>镜</span></div>
      <div><strong>正在本机核对</strong><p id="progress-label">验证凭证 · 读取账户 · 整理账单</p></div>`;
    app.querySelector(".query-card")?.append(panel);
  }
  panel.hidden = false;
}

function hideProgress(): void {
  const panel = app.querySelector<HTMLElement>("#query-progress");
  if (panel) panel.hidden = true;
}

function showInputError(message: string, isError: boolean): void {
  const node = app.querySelector<HTMLDivElement>("#query-error");
  if (!node) return;
  node.hidden = !message;
  node.classList.toggle("notice", !isError);
  node.textContent = message;
}

function normalizeInvokeError(error: unknown): string {
  if (typeof error === "string") return error;
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return "查询没有完成，请检查凭证和网络后重试。";
}

function showLanding(): void {
  if (clearTimer !== undefined) window.clearTimeout(clearTimer);
  app.innerHTML = landingMarkup();
  bindLandingEvents();
  updateVersionLabels();
  window.scrollTo({ top: 0, behavior: "smooth" });
}

function clearResult(): void {
  showLanding();
  const input = credentialInput();
  input?.focus();
}

function renderDashboard(result: InspectionResult): void {
  const content = app.querySelector<HTMLElement>("#main-content");
  if (!content) return;
  const subscription = result.subscription;
  const state = statePresentation(subscription);
  const channel = channelName(subscription.purchaseOrigin);
  const remaining = formatRemaining(subscription.remainingSeconds, subscription.state);
  const records = [...result.mobileRecords, ...result.invoices];

  content.innerHTML = `
    <section class="result-heading">
      <div><p class="eyebrow"><span></span> 本次查询已完成</p><h1>你的订阅全貌</h1><p>查询于 ${escapeHtml(formatDateTime(result.queriedAt))}，结果只保留在当前窗口内。</p></div>
      <div class="result-controls">
        <button class="secondary-button" type="button" data-action="clear-result">清除结果</button>
        <button class="primary-compact" type="button" data-action="new-query">查询另一个账号 <b>→</b></button>
      </div>
    </section>

    <section class="subscription-hero theme-${planTheme(subscription.plan)}">
      <div class="plan-glow" aria-hidden="true"></div>
      <div class="subscription-main">
        <div class="account-line"><span class="status-dot ${state.className}"></span>${escapeHtml(result.identity.email || result.identity.name || "ChatGPT 账号")}</div>
        <p class="status-kicker">${escapeHtml(state.label)}</p>
        <h2>${escapeHtml(subscription.plan)}</h2>
        <p class="subscription-sentence">${escapeHtml(subscriptionSentence(subscription, remaining))}</p>
        <div class="hero-badges">
          <span>${escapeHtml(channel)}</span>
          <span>${escapeHtml(subscription.billingPeriod ? periodName(subscription.billingPeriod) : "周期未知")}</span>
          ${subscription.freeReason ? `<span>${escapeHtml(subscription.freeReason)}</span>` : ""}
        </div>
      </div>
      <div class="time-medallion">
        <small>剩余时间</small><strong>${escapeHtml(remaining.primary)}</strong><span>${escapeHtml(remaining.secondary)}</span>
      </div>
      <dl class="hero-facts">
        ${fact("当前套餐", subscription.plan)}
        ${fact("支付币种", subscription.currency || "—")}
        ${fact("自动续费", renewName(subscription.willRenew, subscription.purchaseOrigin))}
        ${fact("支付渠道", channel)}
      </dl>
    </section>

    ${subscription.delinquent ? delinquentMarkup(subscription) : ""}

    <section class="report-section quota-section">
      ${sectionHeading("01", "额度与恢复时间", "Codex 客户端配额端点本次返回的窗口状态")}
      ${usageMarkup(result.usage)}
    </section>

    <section class="report-section details-section">
      ${sectionHeading("02", "订阅详情", "把账户权益与计费周期交叉核对")}
      <dl class="detail-grid">
        ${detail("账号邮箱", result.identity.email || "—")}
        ${detail("账户 ID", maskId(result.identity.accountId))}
        ${detail("本期开始", formatDateTime(subscription.activeStart))}
        ${detail("本期结束", formatDateTime(subscription.activeUntil))}
        ${detail("套餐周期", subscription.billingPeriod ? periodName(subscription.billingPeriod) : "—")}
        ${detail("当前状态", state.label)}
        ${detail("最近金额", formatMoney(subscription.currentAmount, subscription.currency))}
        ${detail("曾有付费订阅", boolName(result.identity.hasPreviouslyPaid))}
      </dl>
    </section>

    <section class="report-section history-section">
      ${sectionHeading("03", "账单与订阅记录", "网页账单与移动端最近可确认记录分开标注")}
      ${recordsMarkup(records)}
      <div class="coverage-note"><span>范围说明</span><p>${escapeHtml(result.coverage.mobileHistory)}</p></div>
    </section>

    <section class="report-section source-section">
      ${sectionHeading("04", "数据来源与稳定性", "每项结果都标明本次是否真正取得数据")}
      <div class="source-list">${result.sources.map(sourceMarkup).join("")}</div>
      <details class="warning-box">
        <summary>查看 ${result.warnings.length} 条边界与提示</summary>
        <ul>${result.warnings.map((warning) => `<li>${escapeHtml(warning)}</li>`).join("")}</ul>
      </details>
    </section>`;

  content.querySelectorAll<HTMLElement>("[data-action]").forEach((element) => {
    element.addEventListener("click", () => void handleAction(element.dataset.action ?? ""));
  });
  window.scrollTo({ top: 0, behavior: "smooth" });
}

function sectionHeading(index: string, title: string, subtitle: string): string {
  return `<header class="section-heading"><span>${index}</span><div><h2>${escapeHtml(title)}</h2><p>${escapeHtml(subtitle)}</p></div></header>`;
}

function statePresentation(subscription: SubscriptionSummary): { label: string; className: string } {
  if (subscription.delinquent) return { label: "扣款异常 · 宽限期", className: "warning" };
  if (subscription.state === "active") return { label: "正常有效", className: "active" };
  if (subscription.state === "expired") return { label: "订阅已结束", className: "expired" };
  return { label: "当前为免费方案", className: "free" };
}

function subscriptionSentence(subscription: SubscriptionSummary, remaining: { primary: string; secondary: string }): string {
  if (subscription.delinquent) {
    return subscription.gracePeriodEnd
      ? `存在扣款异常，宽限期预计到 ${formatDateTime(subscription.gracePeriodEnd)}。`
      : "存在扣款异常，请在原支付平台核对。";
  }
  if (subscription.state === "active") {
    return subscription.activeUntil
      ? `有效至 ${formatDateTime(subscription.activeUntil)}，${renewName(subscription.willRenew, subscription.purchaseOrigin)}。`
      : `订阅当前有效，${remaining.primary}${remaining.secondary}。`;
  }
  if (subscription.state === "expired") return "历史付费订阅已经结束，当前权益以 Free 为准。";
  return "没有检测到生效中的付费订阅。";
}

function delinquentMarkup(subscription: SubscriptionSummary): string {
  return `<div class="danger-banner"><span>!</span><div><strong>检测到续费扣款异常</strong><p>${subscription.gracePeriodEnd ? `宽限期预计到 ${escapeHtml(formatDateTime(subscription.gracePeriodEnd))}。` : "上游未返回明确宽限期。"}请在原支付平台确认付款状态。</p></div></div>`;
}

function usageMarkup(usage: UsageSummary): string {
  if (usage.status !== "available") {
    return `<div class="empty-state"><span>—</span><div><strong>本次未取得额度数据</strong><p>${escapeHtml(usage.message || "该账号或网络未开放 Codex 额度端点。")}</p></div></div>`;
  }
  if (!usage.windows.length) {
    return `<div class="empty-state"><span>✓</span><div><strong>端点可用，但没有额度窗口</strong><p>Business / Enterprise 的计量方式可能不返回个人滚动窗口。</p></div></div>`;
  }
  return `<div class="usage-grid">${usage.windows.map((window) => {
    const used = clamp(window.usedPercent, 0, 100);
    return `<article class="usage-card">
      <div class="usage-card-head"><div><small>${window.model ? "专项额度" : "可用额度"}</small><h3>${escapeHtml(window.label)}</h3></div><strong>${formatPercent(window.remainingPercent)}<span>剩余</span></strong></div>
      <div class="meter"><i style="width:${used}%"></i></div>
      <div class="usage-meta"><span>已使用 ${formatPercent(used)}</span><span>${window.resetAt ? `恢复于 ${escapeHtml(formatDateTime(window.resetAt))}` : "恢复时间未知"}</span></div>
    </article>`;
  }).join("")}</div>`;
}

function recordsMarkup(records: BillingRecord[]): string {
  if (!records.length) {
    return `<div class="empty-state"><span>∅</span><div><strong>本次没有返回账单记录</strong><p>免费账号、移动商店订阅或内部账单端点变化时可能出现这种情况。</p></div></div>`;
  }
  return `<div class="records-list">${records.map((record) => {
    const mobile = record.recordType === "mobile_last_known";
    return `<article class="record-card ${mobile ? "mobile" : "web"}">
      <div class="store-badge">${mobile ? storeGlyph(record.store) : "W"}</div>
      <div class="record-main"><div class="record-title"><h3>${escapeHtml(record.product)}</h3><span>${escapeHtml(recordStatus(record.status))}</span></div><p>${escapeHtml(mobile ? channelName(record.store) : "ChatGPT 网页账单")}</p></div>
      <div class="record-meta"><small>${mobile ? "订阅区间" : "账单日期"}</small><strong>${escapeHtml(recordDate(record))}</strong></div>
      <div class="record-amount"><strong>${escapeHtml(formatMoney(record.amount, record.currency))}</strong><small>${record.willRenew === undefined ? "" : record.willRenew ? "将自动续费" : "不会自动续费"}</small></div>
      ${record.sourceNote ? `<p class="record-note">${escapeHtml(record.sourceNote)}</p>` : ""}
    </article>`;
  }).join("")}</div>`;
}

function sourceMarkup(source: SourceStatus): string {
  const ok = source.status === "ok";
  return `<article class="source-row"><span class="source-state ${ok ? "ok" : "off"}">${ok ? "✓" : "—"}</span><div><strong>${escapeHtml(source.label)}</strong><p>${escapeHtml(source.detail)}</p></div><span class="source-tag">${source.officialPublicApi ? "公开 API" : "内部只读端点"}</span></article>`;
}

function fact(label: string, value: string): string {
  return `<div><dt>${escapeHtml(label)}</dt><dd>${escapeHtml(value)}</dd></div>`;
}

function detail(label: string, value: string): string {
  return `<div><dt>${escapeHtml(label)}</dt><dd>${escapeHtml(value)}</dd></div>`;
}

function formatRemaining(seconds: number | undefined, state: string): { primary: string; secondary: string } {
  if (state !== "active" && state !== "delinquent") return { primary: "—", secondary: "无生效周期" };
  if (seconds === undefined) return { primary: "有效", secondary: "未返回到期时间" };
  const safe = Math.max(0, seconds);
  const days = Math.floor(safe / 86400);
  const hours = Math.floor((safe % 86400) / 3600);
  if (days > 0) return { primary: `${days} 天`, secondary: `${hours} 小时` };
  const minutes = Math.floor((safe % 3600) / 60);
  return { primary: `${hours} 小时`, secondary: `${minutes} 分钟` };
}

function formatDateTime(value?: string): string {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  }).format(date);
}

function formatMoney(amount?: number, currency?: string): string {
  if (amount === undefined) return currency || "—";
  if (!currency) return amount.toFixed(2);
  try {
    return new Intl.NumberFormat("zh-CN", { style: "currency", currency }).format(amount);
  } catch {
    return `${amount.toFixed(2)} ${currency}`;
  }
}

function channelName(origin?: string): string {
  if (!origin) return "未知渠道";
  const value = origin.toLowerCase();
  if (value.includes("ios") || value.includes("apple") || value.includes("app_store")) return "Apple App Store";
  if (value.includes("android") || value.includes("google") || value.includes("play_store")) return "Google Play";
  if (value.includes("web") || value.includes("stripe")) return "网页 / 卡付";
  if (value.includes("not_purchased")) return "未购买";
  return origin;
}

function renewName(value: boolean | undefined, origin?: string): string {
  if (value === true) return "自动续费已开启";
  if (value === false) return "自动续费已关闭";
  const channel = channelName(origin);
  return channel.includes("Apple") || channel.includes("Google") ? "由移动商店管理" : "续费状态未知";
}

function periodName(value: string): string {
  const normalized = value.toLowerCase();
  if (normalized.includes("month")) return "按月";
  if (normalized.includes("year")) return "按年";
  if (normalized.includes("week")) return "按周";
  return value;
}

function boolName(value?: boolean): string {
  if (value === true) return "是";
  if (value === false) return "否";
  return "—";
}

function maskId(value?: string): string {
  if (!value) return "—";
  if (value.length <= 14) return value;
  return `${value.slice(0, 7)}•••${value.slice(-5)}`;
}

function recordDate(record: BillingRecord): string {
  if (record.periodStart || record.periodEnd) {
    return `${formatShortDate(record.periodStart)} → ${formatShortDate(record.periodEnd)}`;
  }
  return formatDateTime(record.createdAt);
}

function formatShortDate(value?: string): string {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", { year: "numeric", month: "2-digit", day: "2-digit" }).format(date);
}

function recordStatus(status: string): string {
  const value = status.toLowerCase();
  if (["paid", "succeeded", "active"].includes(value)) return "已支付 / 有效";
  if (["open", "pending", "draft"].includes(value)) return "待处理";
  if (["refunded", "void"].includes(value)) return "已退款 / 作废";
  if (value === "expired") return "已结束";
  if (value === "delinquent") return "扣款异常";
  return status || "未知";
}

function storeGlyph(store?: string): string {
  return channelName(store).startsWith("Apple") ? "A" : "G";
}

function planTheme(plan: string): string {
  const value = plan.toLowerCase();
  if (value.includes("pro")) return "pro";
  if (value.includes("plus")) return "plus";
  if (value.includes("business") || value.includes("team")) return "team";
  return "free";
}

function formatPercent(value: number): string {
  return `${Math.round(value * 10) / 10}%`;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function toggleTheme(): void {
  const root = document.documentElement;
  const next = root.dataset.theme === "dark" ? "light" : "dark";
  root.dataset.theme = next;
}

function escapeHtml(value: unknown): string {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}
