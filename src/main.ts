import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

type AppInfo = {
  name: string;
  version: string;
  localOnly: boolean;
  readOnly: boolean;
  allowedHosts: string[];
};

type CredentialCheck = {
  id: string;
  label: string;
  status: "pass" | "fail" | "pending";
  detail: string;
};

type CredentialValidation = {
  kind: string;
  valid: boolean;
  complete: boolean;
  canQuery: boolean;
  summary: string;
  checks: CredentialCheck[];
  missing: string[];
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

type PaymentMethodSummary = {
  kind: string;
  brand: string;
  first6?: string;
  last4?: string;
  expMonth?: number;
  expYear?: number;
  isDefault: boolean;
  sourceNote: string;
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
  paymentMethods: PaymentMethodSummary[];
  sources: SourceStatus[];
  warnings: string[];
  coverage: {
    currentSubscription: string;
    webInvoiceHistory: string;
    mobileHistory: string;
    quota: string;
  };
};

type PaymentPresentation = {
  kind: "visa" | "mastercard" | "appstore" | "googleplay" | "card" | "free";
  title: string;
  number: string;
  meta: string;
  note: string;
};

const BRAND_PATHS = {
  visa: "M9.112 8.262 5.97 15.758H3.92L2.374 9.775c-.094-.368-.175-.503-.461-.658C1.447 8.864.677 8.627 0 8.479l.046-.217h3.3a.904.904 0 0 1 .894.764l.817 4.338 2.018-5.102zm8.033 5.049c.008-1.979-2.736-2.088-2.717-2.972.006-.269.262-.555.822-.628a3.66 3.66 0 0 1 1.913.336l.34-1.59a5.207 5.207 0 0 0-1.814-.333c-1.917 0-3.266 1.02-3.278 2.479-.012 1.079.963 1.68 1.698 2.04.756.367 1.01.603 1.006.931-.005.504-.602.725-1.16.734-.975.015-1.54-.263-1.992-.473l-.351 1.642c.453.208 1.289.39 2.156.398 2.037 0 3.37-1.006 3.377-2.564m5.061 2.447H24l-1.565-7.496h-1.656a.883.883 0 0 0-.826.55l-2.909 6.946h2.036l.405-1.12h2.488zm-2.163-2.656 1.02-2.815.588 2.815zm-8.16-4.84-1.603 7.496H8.34l1.605-7.496z",
  mastercard: "M11.343 18.031c.058.049.12.098.181.146-1.177.783-2.59 1.238-4.107 1.238C3.32 19.416 0 16.096 0 12c0-4.095 3.32-7.416 7.416-7.416 1.518 0 2.931.456 4.105 1.238-.06.051-.12.098-.165.15C9.6 7.489 8.595 9.688 8.595 12c0 2.311 1.001 4.51 2.748 6.031zm5.241-13.447c-1.52 0-2.931.456-4.105 1.238.06.051.12.098.165.15C14.4 7.489 15.405 9.688 15.405 12c0 2.31-1.001 4.507-2.748 6.031-.058.049-.12.098-.181.146 1.177.783 2.588 1.238 4.107 1.238C20.68 19.416 24 16.096 24 12c0-4.094-3.32-7.416-7.416-7.416zM12 6.174c-.096.075-.189.15-.28.231C10.156 7.764 9.169 9.765 9.169 12c0 2.236.987 4.236 2.551 5.595.09.08.185.158.28.232.096-.074.189-.152.28-.232 1.563-1.359 2.551-3.359 2.551-5.595 0-2.235-.987-4.236-2.551-5.595-.09-.08-.184-.156-.28-.231z",
  appstore: "M8.8086 14.9194l6.1107-11.0368c.0837-.1513.1682-.302.2437-.4584.0685-.142.1267-.2854.1646-.4403.0803-.3259.0588-.6656-.066-.9767-.1238-.3095-.3417-.5678-.6201-.7355a1.4175 1.4175 0 0 0-.921-.1924c-.3207.043-.6135.1935-.8443.4288-.1094.1118-.1996.2361-.2832.369-.092.1463-.175.2979-.259.4492l-.3864.6979-.3865-.6979c-.0837-.1515-.1667-.303-.2587-.4492-.0837-.1329-.1739-.2572-.2835-.369-.2305-.2353-.5233-.3857-.844-.429a1.4181 1.4181 0 0 0-.921.1926c-.2784.1677-.4964.426-.6203.7355-.1246.311-.1461.6508-.066.9767.038.155.0962.2984.1648.4403.0753.1564.1598.307.2437.4584l1.248 2.2543-4.8625 8.7825H2.0295c-.1676 0-.3351-.0007-.5026.0092-.1522.009-.3004.0284-.448.0714-.3108.0906-.5822.2798-.7783.548-.195.2665-.3006.5929-.3006.9279 0 .3352.1057.6612.3006.9277.196.2683.4675.4575.7782.548.1477.043.296.0623.4481.0715.1675.01.335.009.5026.009h13.0974c.0171-.0357.059-.1294.1-.2697.415-1.4151-.6156-2.843-2.0347-2.843zM3.113 18.5418l-.7922 1.5008c-.0818.1553-.1644.31-.2384.4705-.067.1458-.124.293-.1611.452-.0785.3346-.0576.6834.0645 1.0029.1212.3175.3346.583.607.7549.2727.172.5891.2416.9013.1975.3139-.044.6005-.1986.8263-.4402.1072-.1148.1954-.2424.2772-.3787.0902-.1503.1714-.3059.2535-.4612L6 19.4636c-.0896-.149-.9473-1.4704-2.887-.9218m20.5861-3.0056a1.4707 1.4707 0 0 0-.779-.5407c-.1476-.0425-.2961-.0616-.4483-.0705-.1678-.0099-.3352-.0091-.503-.0091H18.648l-4.3891-7.817c-.6655.7005-.9632 1.485-1.0773 2.1976-.1655 1.0333.0367 2.0934.546 3.0004l5.2741 9.3933c.084.1494.167.299.2591.4435.0837.131.1739.2537.2836.364.231.2323.5238.3809.8449.4232.3192.0424.643-.0244.9217-.1899.2784-.1653.4968-.4204.621-.7257.1246-.3072.146-.6425.0658-.9641-.0381-.1529-.0962-.2945-.165-.4346-.0753-.1543-.1598-.303-.2438-.4524l-1.216-2.1662h1.596c.1677 0 .3351.0009.5029-.009.1522-.009.3007-.028.4483-.0705a1.4707 1.4707 0 0 0 .779-.5407A1.5386 1.5386 0 0 0 24 16.452a1.539 1.539 0 0 0-.3009-.9158Z",
  googleplay: "M22.018 13.298l-3.919 2.218-3.515-3.493 3.543-3.521 3.891 2.202a1.49 1.49 0 0 1 0 2.594zM1.337.924a1.486 1.486 0 0 0-.112.568v21.017c0 .217.045.419.124.6l11.155-11.087L1.337.924zm12.207 10.065 3.258-3.238L3.45.195a1.466 1.466 0 0 0-.946-.179l11.04 10.973zm0 2.067-11 10.933c.298.036.612-.016.906-.183l13.324-7.54-3.23-3.21z",
} as const;

const appElement = document.querySelector<HTMLDivElement>("#app");
if (!appElement) throw new Error("Missing #app");
const app = appElement;

let appInfo: AppInfo = {
  name: "订阅镜",
  version: "1.1.0",
  localOnly: true,
  readOnly: true,
  allowedHosts: ["chatgpt.com"],
};
let clearTimer: number | undefined;
let validationTimer: number | undefined;
let currentValidation: CredentialValidation | null = null;
let queryBusy = false;

app.innerHTML = landingMarkup();
bindLandingEvents();
void loadAppInfo();

function landingMarkup(): string {
  return `
    <div class="desktop-shell">
      ${sidebarMarkup("credential")}
      <section class="workspace">
        <header class="tool-bar">
          <div><span class="connection-dot"></span>本机模式</div>
          <div class="tool-meta"><span>只读</span><span>无遥测</span><span>v<span data-version>1.1.0</span></span></div>
        </header>

        <main class="work-area">
          <section class="page-heading">
            <p>本地 · 只读 · 无中转</p>
            <h1>把订阅信息，<br /><em>留在这台电脑里。</em></h1>
            <span>粘贴完整会话，先在本机确认结构，再读取你的订阅与账单信息。</span>
          </section>

          <section class="input-workbench" aria-labelledby="credential-title">
            <header class="workbench-header">
              <div><span class="section-kicker">会话</span><div><h2 id="credential-title">从这里开始</h2><p>支持完整 Session JSON、Codex auth.json 或 Session Token</p></div></div>
              <span class="memory-label">仅在内存中处理</span>
            </header>

            <div class="editor-wrap">
              <label for="credential-input">输入内容</label>
              <textarea id="credential-input" rows="11" autocomplete="off" autocapitalize="off" spellcheck="false" placeholder='请粘贴完整 JSON，例如：{ "user": { ... }, "account": { ... }, "accessToken": "eyJ..." }'></textarea>
              <span id="character-count" class="byte-count">0 B</span>
            </div>
            <div class="editor-actions">
              <button type="button" data-action="paste">从剪贴板粘贴</button>
              <button type="button" data-action="clear-input">清空输入</button>
            </div>

            <section class="check-section">
              <header><span class="section-kicker">校验</span><div><h2>会话是否完整？</h2><p>结构、身份、账户范围与有效期均需通过</p></div></header>
              <div id="validation-panel" class="validation-panel" aria-live="polite">${emptyValidationMarkup()}</div>
            </section>

            <div class="submit-section">
              <label class="ownership-row">
                <input id="ownership-confirm" type="checkbox" />
                <span>这是我自己的账号，或账号所有者已明确授权我查询。</span>
              </label>
              <div id="query-error" class="query-error" role="alert" hidden></div>
              <button id="query-button" class="query-button" type="button" data-action="query" disabled>
                <span>验证并查看订阅</span><b>→</b>
              </button>
            </div>
          </section>

          <aside class="local-note">
            <strong>数据边界</strong>
            <p>凭证不会发送给本项目作者。程序仅直接连接 <code>chatgpt.com</code>；成功查询后立即清空输入，未查询内容在 5 分钟后清空。</p>
          </aside>
        </main>

        <footer class="status-bar"><span>MIT 开源</span><span>网络目标：chatgpt.com</span><span>Windows x64</span></footer>
      </section>
    </div>`;
}

function sidebarMarkup(active: "credential" | "result"): string {
  return `<aside class="side-bar">
    <button class="app-identity" type="button" data-action="home" aria-label="返回查询页">
      <i class="app-mark" aria-hidden="true"></i><span class="identity-copy"><strong>订阅镜</strong><small>Subscription Lens</small></span>
    </button>
    <nav aria-label="查询工作区">
      <p>查询流程</p>
      <div class="nav-item ${active === "credential" ? "active" : "done"}"><i></i><span>输入会话</span></div>
      <div class="nav-item ${active === "credential" ? "active-sub" : "done"}"><i></i><span>完整性校验</span></div>
      <div class="nav-item ${active === "result" ? "active" : ""}"><i></i><span>查询结果</span></div>
    </nav>
    <div class="side-security"><i>✓</i><div><strong>本机私密模式</strong><span>凭证不落盘，也不经过作者服务器</span></div></div>
  </aside>`;
}

function bindLandingEvents(): void {
  app.querySelectorAll<HTMLElement>("[data-action]").forEach((element) => {
    element.addEventListener("click", () => void handleAction(element.dataset.action ?? ""));
  });
  const input = credentialInput();
  input?.addEventListener("input", () => {
    scheduleInputClear();
    updateCharacterCount();
    scheduleCredentialValidation();
  });
  input?.addEventListener("keydown", (event) => {
    if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
      event.preventDefault();
      void runInspection();
    }
  });
  app.querySelector<HTMLInputElement>("#ownership-confirm")?.addEventListener("change", updateQueryButton);
  updateQueryButton();
}

async function handleAction(action: string): Promise<void> {
  if (action === "paste") await pasteCredential();
  if (action === "clear-input") clearCredential();
  if (action === "query") await runInspection();
  if (action === "home" || action === "new-query" || action === "clear-result") showLanding();
}

async function loadAppInfo(): Promise<void> {
  try {
    appInfo = await invoke<AppInfo>("app_info");
  } catch {
    // Browser preview uses the embedded metadata above.
  }
  document.querySelectorAll<HTMLElement>("[data-version]").forEach((node) => {
    node.textContent = appInfo.version;
  });
}

function credentialInput(): HTMLTextAreaElement | null {
  return app.querySelector<HTMLTextAreaElement>("#credential-input");
}

function scheduleInputClear(): void {
  if (clearTimer !== undefined) window.clearTimeout(clearTimer);
  const input = credentialInput();
  if (!input?.value) return;
  clearTimer = window.setTimeout(() => {
    clearCredential();
    showQueryError("为保护凭证，闲置 5 分钟后输入已自动清空。", false);
  }, 5 * 60 * 1000);
}

function clearCredential(): void {
  const input = credentialInput();
  if (input) input.value = "";
  if (clearTimer !== undefined) window.clearTimeout(clearTimer);
  if (validationTimer !== undefined) window.clearTimeout(validationTimer);
  clearTimer = undefined;
  validationTimer = undefined;
  currentValidation = null;
  updateCharacterCount();
  renderValidation(null);
  showQueryError("", false);
  updateQueryButton();
}

async function pasteCredential(): Promise<void> {
  try {
    const value = await navigator.clipboard.readText();
    const input = credentialInput();
    if (!input) return;
    input.value = value.trim();
    input.focus();
    scheduleInputClear();
    updateCharacterCount();
    await validateCredentialNow();
  } catch {
    showQueryError("无法读取剪贴板，请使用 Ctrl+V 粘贴。", true);
  }
}

function updateCharacterCount(): void {
  const count = new TextEncoder().encode(credentialInput()?.value ?? "").length;
  const node = app.querySelector<HTMLElement>("#character-count");
  if (node) node.textContent = count < 1024 ? `${count} B` : `${(count / 1024).toFixed(1)} KiB`;
}

function scheduleCredentialValidation(): void {
  if (validationTimer !== undefined) window.clearTimeout(validationTimer);
  currentValidation = null;
  renderValidationLoading();
  updateQueryButton();
  validationTimer = window.setTimeout(() => void validateCredentialNow(), 220);
}

async function validateCredentialNow(): Promise<CredentialValidation | null> {
  const value = credentialInput()?.value.trim() ?? "";
  if (!value) {
    currentValidation = null;
    renderValidation(null);
    updateQueryButton();
    return null;
  }
  try {
    currentValidation = await invoke<CredentialValidation>("validate_credential", { credential: value });
  } catch {
    currentValidation = browserPreviewValidation(value);
  }
  renderValidation(currentValidation);
  updateQueryButton();
  return currentValidation;
}

function browserPreviewValidation(value: string): CredentialValidation {
  const jsonLike = value.trim().startsWith("{");
  let usable = value.trim().length >= 32;
  if (jsonLike) {
    try {
      const parsed = JSON.parse(value) as Record<string, unknown>;
      usable = Boolean(parsed.accessToken || parsed.access_token || parsed.sessionToken || parsed.tokens);
    } catch {
      usable = false;
    }
  }
  return {
    kind: jsonLike ? "session_json" : "session_token",
    valid: usable,
    complete: usable,
    canQuery: usable,
    summary: usable ? "浏览器预览：格式检查通过" : "输入看起来不完整",
    checks: [
      { id: "format", label: "凭证结构", status: usable ? "pass" : "fail", detail: usable ? "格式可识别" : "缺少完整会话内容" },
    ],
    missing: usable ? [] : ["完整会话内容"],
  };
}

function renderValidation(validation: CredentialValidation | null): void {
  const node = app.querySelector<HTMLElement>("#validation-panel");
  if (!node) return;
  if (!validation) {
    node.innerHTML = emptyValidationMarkup();
    node.dataset.state = "empty";
    return;
  }
  node.dataset.state = validation.canQuery ? (validation.complete ? "pass" : "pending") : "fail";
  node.innerHTML = `
    <div class="check-summary">
      <span class="check-symbol">${validation.canQuery ? (validation.complete ? "✓" : "…") : "!"}</span>
      <div><strong>${escapeHtml(validation.summary)}</strong><small>${escapeHtml(credentialKindName(validation.kind))}</small></div>
    </div>
    <div class="check-list">
      ${validation.checks.map((check) => `
        <div class="check-row state-${check.status}">
          <i>${check.status === "pass" ? "✓" : check.status === "fail" ? "×" : "—"}</i>
          <span><b>${escapeHtml(check.label)}</b><small>${escapeHtml(check.detail)}</small></span>
        </div>`).join("")}
    </div>`;
}

function emptyValidationMarkup(): string {
  return `<div class="check-placeholder"><span>—</span><p><strong>等待会话数据</strong><small>将检查结构、身份、账户范围和有效期</small></p></div>`;
}

function renderValidationLoading(): void {
  const node = app.querySelector<HTMLElement>("#validation-panel");
  if (!node) return;
  node.dataset.state = "loading";
  node.innerHTML = `<div class="check-placeholder"><span class="spinner"></span><p><strong>正在检查</strong><small>本机解析，不会发送给第三方</small></p></div>`;
}

function credentialKindName(kind: string): string {
  const names: Record<string, string> = {
    session_json: "完整 Session JSON",
    codex_auth_json: "Codex auth.json",
    access_token: "Access Token",
    session_token: "Session Token · 将先联网确认完整性",
  };
  return names[kind] ?? "会话凭证";
}

function updateQueryButton(): void {
  const button = app.querySelector<HTMLButtonElement>("#query-button");
  const consent = app.querySelector<HTMLInputElement>("#ownership-confirm");
  if (!button) return;
  button.disabled = queryBusy || !currentValidation?.canQuery || !consent?.checked;
}

async function runInspection(): Promise<void> {
  const input = credentialInput();
  const consent = app.querySelector<HTMLInputElement>("#ownership-confirm");
  const value = input?.value.trim() ?? "";
  if (!value) {
    showQueryError("请先粘贴完整的 Session JSON 或 Session Token。", true);
    input?.focus();
    return;
  }
  const validation = await validateCredentialNow();
  if (!validation?.canQuery) {
    const suffix = validation?.missing.length ? `缺少：${validation.missing.join("、")}。` : "";
    showQueryError(`凭证不完整，已阻止查询。${suffix}`, true);
    input?.focus();
    return;
  }
  if (!consent?.checked) {
    showQueryError("请先确认你有权查询这个账号。", true);
    consent?.focus();
    return;
  }

  showQueryError("", false);
  setQueryBusy(true);
  try {
    const result = await invoke<InspectionResult>("inspect_subscription", {
      request: { credential: value, timezoneOffsetMin: new Date().getTimezoneOffset() },
    });
    clearCredential();
    renderDashboard(result);
  } catch (error) {
    showQueryError(normalizeInvokeError(error), true);
  } finally {
    setQueryBusy(false);
  }
}

function setQueryBusy(busy: boolean): void {
  queryBusy = busy;
  const button = app.querySelector<HTMLButtonElement>("#query-button");
  if (button) {
    button.innerHTML = busy
      ? '<span><i class="button-spinner"></i> 正在验证并查询</span><b>···</b>'
      : "<span>验证并查询</span><b>↗</b>";
  }
  const validation = app.querySelector<HTMLElement>("#validation-panel");
  if (busy && validation) {
    validation.dataset.state = "loading";
    validation.innerHTML = `<div class="check-placeholder"><span class="spinner"></span><p><strong>正在验证完整会话</strong><small>通过后才会读取订阅数据</small></p></div>`;
  }
  updateQueryButton();
}

function showQueryError(message: string, isError: boolean): void {
  const node = app.querySelector<HTMLElement>("#query-error");
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
  return "查询没有完成，请检查凭证完整性和网络后重试。";
}

function showLanding(): void {
  if (clearTimer !== undefined) window.clearTimeout(clearTimer);
  if (validationTimer !== undefined) window.clearTimeout(validationTimer);
  clearTimer = undefined;
  validationTimer = undefined;
  currentValidation = null;
  queryBusy = false;
  app.innerHTML = landingMarkup();
  bindLandingEvents();
  document.querySelectorAll<HTMLElement>("[data-version]").forEach((node) => {
    node.textContent = appInfo.version;
  });
  window.scrollTo({ top: 0, behavior: "auto" });
}

function renderDashboard(result: InspectionResult): void {
  const subscription = result.subscription;
  const state = statePresentation(subscription);
  const remaining = formatRemaining(subscription.remainingSeconds, subscription.state);
  const payment = paymentPresentation(subscription, result.paymentMethods);
  const records = [...result.mobileRecords, ...result.invoices];

  app.innerHTML = `
    <div class="desktop-shell">
      ${sidebarMarkup("result")}
      <section class="workspace">
        <header class="tool-bar">
          <div><span class="connection-dot"></span>查询完成</div>
          <div class="tool-meta"><span>${escapeHtml(formatDateTime(result.queriedAt))}</span><span>结果未保存</span></div>
        </header>

        <main class="work-area result-work-area">
          <section class="result-heading">
            <div><p>当前账户</p><h1>${escapeHtml(result.identity.email || result.identity.name || "ChatGPT 账号")}</h1></div>
            <div class="result-actions"><button type="button" data-action="clear-result">清除</button><button class="primary" type="button" data-action="new-query">新查询</button></div>
          </section>

          <section class="account-overview">
            <header>
              <span class="status-label status-${state.className}"><i></i>${escapeHtml(state.label)}</span>
              <p>${escapeHtml(subscriptionSentence(subscription))}</p>
            </header>
            <div class="overview-values">
              <div><span>当前套餐</span><strong>${escapeHtml(subscription.plan)}</strong></div>
              <div><span>剩余时间</span><strong>${escapeHtml(remaining.primary)} <small>${escapeHtml(remaining.secondary)}</small></strong></div>
              <div><span>到期时间</span><strong>${escapeHtml(formatDateTime(subscription.activeUntil))}</strong></div>
            </div>
            <div class="payment-row payment-${payment.kind}">
              <div class="payment-logo">${paymentIcon(payment.kind)}</div>
              <div><span>支付方式</span><strong>${escapeHtml(payment.title)}</strong><small>${escapeHtml(payment.meta)}</small></div>
              <code>${escapeHtml(payment.number)}</code>
              <p>${escapeHtml(payment.note)}</p>
            </div>
          </section>

          ${subscription.delinquent ? delinquentMarkup(subscription) : ""}

          <section class="utility-section">
            ${sectionHeader("账户与订阅", "由账户权益、订阅周期和账单信息汇总")}
            <dl class="property-table">
              ${dataItem("账号邮箱", result.identity.email || "—")}
              ${dataItem("账户 ID", maskId(result.identity.accountId))}
              ${dataItem("本期开始", formatDateTime(subscription.activeStart))}
              ${dataItem("本期结束", formatDateTime(subscription.activeUntil))}
              ${dataItem("计费周期", subscription.billingPeriod ? periodName(subscription.billingPeriod) : "—")}
              ${dataItem("支付币种", subscription.currency || "—")}
              ${dataItem("最近金额", formatMoney(subscription.currentAmount, subscription.currency))}
              ${dataItem("支付渠道", channelName(subscription.purchaseOrigin, result.paymentMethods.length > 0))}
              ${dataItem("自动续费", renewName(subscription.willRenew, subscription.purchaseOrigin))}
              ${dataItem("曾有付费", boolName(result.identity.hasPreviouslyPaid))}
            </dl>
          </section>

          <section class="utility-section">
            ${sectionHeader("Codex 额度", "上游本次返回的滚动额度窗口")}
            ${usageMarkup(result.usage)}
          </section>

          <section class="utility-section">
            ${sectionHeader("账单记录", "网页账单与移动商店最近可确认记录")}
            ${recordsMarkup(records)}
            <div class="scope-line"><strong>覆盖范围</strong><span>${escapeHtml(result.coverage.mobileHistory)}</span></div>
          </section>

          <section class="utility-section">
            ${sectionHeader("数据来源", "每个接口独立显示本次状态")}
            <div class="source-table">${result.sources.map(sourceMarkup).join("")}</div>
            <details class="warning-list" ${result.warnings.length ? "" : "hidden"}>
              <summary>边界与提示（${result.warnings.length}）</summary>
              <ul>${result.warnings.map((warning) => `<li>${escapeHtml(warning)}</li>`).join("")}</ul>
            </details>
          </section>
        </main>

        <footer class="status-bar"><span>只读查询</span><span>结果仅存在当前窗口</span><span>v${escapeHtml(appInfo.version)}</span></footer>
      </section>
    </div>`;

  app.querySelectorAll<HTMLElement>("[data-action]").forEach((element) => {
    element.addEventListener("click", () => void handleAction(element.dataset.action ?? ""));
  });
  window.scrollTo({ top: 0, behavior: "auto" });
}

function statePresentation(subscription: SubscriptionSummary): { label: string; className: string } {
  if (subscription.delinquent) return { label: "扣款异常", className: "warning" };
  if (subscription.state === "active") return { label: "订阅有效", className: "active" };
  if (subscription.state === "expired") return { label: "订阅已结束", className: "expired" };
  return { label: "免费方案", className: "free" };
}

function subscriptionSentence(subscription: SubscriptionSummary): string {
  if (subscription.delinquent) return "检测到扣款异常，请到原支付平台核对。";
  if (subscription.state === "active" && subscription.activeUntil) return `有效至 ${formatDateTime(subscription.activeUntil)}。`;
  if (subscription.state === "active") return "当前订阅有效，上游未提供明确结束时间。";
  if (subscription.state === "expired") return "历史付费订阅已经结束。";
  return "当前没有检测到生效中的付费订阅。";
}

function paymentPresentation(subscription: SubscriptionSummary, methods: PaymentMethodSummary[]): PaymentPresentation {
  const origin = (subscription.purchaseOrigin ?? "").toLowerCase();
  if (origin.includes("ios") || origin.includes("apple") || origin.includes("app_store")) {
    return {
      kind: "appstore", title: "Apple App Store", number: "iOS 商店订阅",
      meta: subscription.currency ? `结算币种 ${subscription.currency}` : "由 Apple 管理付款",
      note: "完整收据和付款方式需要在 Apple 账户中查看。",
    };
  }
  if (origin.includes("google") || origin.includes("android") || origin.includes("play_store")) {
    return {
      kind: "googleplay", title: "Google Play", number: "Android 商店订阅",
      meta: subscription.currency ? `结算币种 ${subscription.currency}` : "由 Google Play 管理付款",
      note: "完整收据和付款方式需要在 Google Play 账户中查看。",
    };
  }
  const method = methods.find((item) => item.isDefault) ?? methods[0];
  if (method) {
    const kind = method.brand === "visa" ? "visa" : method.brand === "mastercard" ? "mastercard" : "card";
    const title = kind === "visa" ? "Visa" : kind === "mastercard" ? "Mastercard" : cardBrandName(method.brand);
    const visibleNumber = method.first6 && method.last4
      ? `${method.first6} •••••• ${method.last4}`
      : method.last4 ? `•••••• •••••• ${method.last4}` : method.first6 ? `${method.first6} •••••• ••••` : "卡号掩码不可用";
    const expiry = method.expMonth && method.expYear ? `有效期 ${String(method.expMonth).padStart(2, "0")}/${method.expYear}` : "网页银行卡支付";
    return {
      kind, title, number: visibleNumber, meta: expiry,
      note: method.first6 ? "只显示前 6 位和尾号 4 位。" : "上游只返回了尾号，没有提供前 6 位。",
    };
  }
  if (subscription.state === "free") {
    return { kind: "free", title: "无需支付", number: "FREE", meta: "当前免费方案", note: "没有生效中的付费方式。" };
  }
  return {
    kind: "card", title: "网页支付", number: "付款卡信息不可用",
    meta: subscription.currency ? `结算币种 ${subscription.currency}` : "支付渠道未返回",
    note: "ChatGPT 本次没有返回可安全展示的卡品牌或卡号掩码。",
  };
}

function paymentIcon(kind: PaymentPresentation["kind"]): string {
  if (kind === "visa" || kind === "mastercard" || kind === "appstore" || kind === "googleplay") {
    const title = kind === "visa" ? "Visa" : kind === "mastercard" ? "Mastercard" : kind === "appstore" ? "Apple App Store" : "Google Play";
    return `<svg class="brand-svg" role="img" aria-label="${title}" viewBox="0 0 24 24"><path d="${BRAND_PATHS[kind]}"></path></svg>`;
  }
  if (kind === "free") return `<span class="free-symbol">0</span>`;
  return `<svg class="card-svg" aria-hidden="true" viewBox="0 0 48 32"><rect x="1" y="1" width="46" height="30" rx="4"></rect><path d="M1 9h46M7 23h12"></path></svg>`;
}

function cardBrandName(brand: string): string {
  const names: Record<string, string> = { amex: "American Express", unionpay: "UnionPay", jcb: "JCB", discover: "Discover" };
  return names[brand] ?? "银行卡";
}

function sectionHeader(title: string, description: string): string {
  return `<header class="section-header"><h2>${escapeHtml(title)}</h2><p>${escapeHtml(description)}</p></header>`;
}

function dataItem(label: string, value: string): string {
  return `<div><dt>${escapeHtml(label)}</dt><dd>${escapeHtml(value)}</dd></div>`;
}

function delinquentMarkup(subscription: SubscriptionSummary): string {
  const grace = subscription.gracePeriodEnd ? `宽限期预计到 ${formatDateTime(subscription.gracePeriodEnd)}。` : "上游未返回明确宽限期。";
  return `<aside class="issue-row"><b>扣款异常</b><p>${escapeHtml(grace)}请在原支付平台确认。</p></aside>`;
}

function usageMarkup(usage: UsageSummary): string {
  if (usage.status !== "available") {
    return emptyStateMarkup("本次未取得额度数据", usage.message || "该账号或网络没有返回 Codex 额度窗口。");
  }
  if (!usage.windows.length) {
    return emptyStateMarkup("额度端点可用", "本次没有个人滚动窗口。");
  }
  return `<div class="usage-table">${usage.windows.map((window) => {
    const used = clamp(window.usedPercent, 0, 100);
    return `<div class="usage-row">
      <div class="usage-name"><strong>${escapeHtml(window.label)}</strong><span>${window.model ? escapeHtml(window.model) : "滚动额度窗口"}</span></div>
      <div class="usage-meter"><span><i style="width:${used}%"></i></span><small>已用 ${formatPercent(used)}</small></div>
      <div class="usage-remaining"><strong>${formatPercent(window.remainingPercent)}</strong><span>剩余</span></div>
      <time>${window.resetAt ? escapeHtml(formatDateTime(window.resetAt)) : "恢复时间未知"}</time>
    </div>`;
  }).join("")}</div>`;
}

function recordsMarkup(records: BillingRecord[]): string {
  if (!records.length) {
    return emptyStateMarkup("没有返回可展示记录", "这不等于商店账户中没有历史订单。");
  }
  return `<div class="record-table">
    <div class="record-table-head"><span>来源 / 项目</span><span>计费周期</span><span>状态</span><span>金额</span></div>
    ${records.map((record) => `
    <div class="record-row">
      <div class="record-icon">${recordIcon(record.store)}</div>
      <div class="record-main"><strong>${escapeHtml(record.product)}</strong><span>${escapeHtml(record.recordType === "mobile_last_known" ? "移动商店最近记录" : "网页账单")}${record.sourceNote ? ` · ${escapeHtml(record.sourceNote)}` : ""}</span></div>
      <div class="record-period">${escapeHtml(formatShortDate(record.periodStart))} — ${escapeHtml(formatShortDate(record.periodEnd))}</div>
      <div class="record-status">${escapeHtml(recordStatus(record.status))}</div>
      <div class="record-amount">${escapeHtml(formatMoney(record.amount, record.currency))}</div>
    </div>`).join("")}</div>`;
}

function emptyStateMarkup(title: string, detail: string): string {
  return `<div class="empty-state"><span>—</span><div><strong>${escapeHtml(title)}</strong><p>${escapeHtml(detail)}</p></div></div>`;
}

function recordIcon(store?: string): string {
  const value = (store ?? "").toLowerCase();
  if (value.includes("ios") || value.includes("apple") || value.includes("app_store")) return paymentIcon("appstore");
  if (value.includes("google") || value.includes("android") || value.includes("play_store")) return paymentIcon("googleplay");
  return `<span class="invoice-symbol">#</span>`;
}

function sourceMarkup(source: SourceStatus): string {
  const ok = source.status === "ok";
  return `<div class="source-row"><span class="source-state ${ok ? "ok" : "off"}">${ok ? "可用" : "未取到"}</span><strong>${escapeHtml(source.label)}</strong><p>${escapeHtml(source.detail)}</p></div>`;
}

function formatRemaining(seconds: number | undefined, state: string): { primary: string; secondary: string } {
  if (state === "free") return { primary: "—", secondary: "免费方案" };
  if (state === "expired") return { primary: "0", secondary: "已结束" };
  if (seconds === undefined) return { primary: "—", secondary: "未知" };
  const safe = Math.max(0, seconds);
  const days = Math.floor(safe / 86400);
  const hours = Math.floor((safe % 86400) / 3600);
  return days > 0 ? { primary: `${days}`, secondary: `天 ${hours} 小时` } : { primary: `${hours}`, secondary: "小时" };
}

function formatDateTime(value?: string): string {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit", second: "2-digit", hour12: false,
  }).format(date);
}

function formatShortDate(value?: string): string {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", { year: "numeric", month: "2-digit", day: "2-digit" }).format(date);
}

function formatMoney(amount?: number, currency?: string): string {
  if (amount === undefined) return currency || "—";
  try {
    return new Intl.NumberFormat("zh-CN", { style: "currency", currency: currency || "USD" }).format(amount);
  } catch {
    return `${amount.toFixed(2)} ${currency || ""}`.trim();
  }
}

function channelName(origin?: string, hasCard = false): string {
  const value = (origin ?? "").toLowerCase();
  if (value.includes("ios") || value.includes("apple") || value.includes("app_store")) return "Apple App Store";
  if (value.includes("google") || value.includes("android") || value.includes("play_store")) return "Google Play";
  if (hasCard || value.includes("web") || value.includes("stripe") || value.includes("card")) return "网页银行卡";
  return origin || "—";
}

function renewName(value?: boolean, origin?: string): string {
  if (value === true) return "已开启";
  if (value === false) return "已关闭";
  if ((origin ?? "").toLowerCase().includes("apple")) return "由 Apple 管理";
  return "—";
}

function periodName(value: string): string {
  const normalized = value.toLowerCase();
  if (normalized.includes("month")) return "按月";
  if (normalized.includes("year") || normalized.includes("annual")) return "按年";
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
  if (value.length <= 12) return value;
  return `${value.slice(0, 6)}…${value.slice(-4)}`;
}

function recordStatus(status: string): string {
  const value = status.toLowerCase();
  if (["paid", "succeeded", "complete", "active"].includes(value)) return "已支付";
  if (["refunded", "refund"].includes(value)) return "已退款";
  if (["failed", "void", "uncollectible"].includes(value)) return "未完成";
  if (["expired", "canceled", "cancelled"].includes(value)) return "已结束";
  return status || "状态未知";
}

function formatPercent(value: number): string {
  return `${Math.round(clamp(value, 0, 100))}%`;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function escapeHtml(value: unknown): string {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}
