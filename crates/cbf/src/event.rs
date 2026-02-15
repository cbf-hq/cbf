use cursor_icon::CursorIcon;

use crate::data::{
    context_menu::ContextMenu, drag::DragStartRequest, ids::WebPageId, ime::ImeBoundsUpdate,
    profile::ProfileInfo, surface::SurfaceHandle,
};
use crate::error::BackendErrorInfo;

/// Events emitted by the browser backend as a whole.
/// ブラウザエンジン全体から発生するイベントを表します。
#[derive(Debug)]
pub enum BrowserEvent {
    /// The backend is connected and ready to accept commands.
    /// バックエンドへの接続が確立し、コマンド受付やイベント送出が可能になった。
    BackendReady { backend_name: String },

    /// The backend stopped due to shutdown, disconnect, or crash.
    /// バックエンドが停止した（終了、切断、クラッシュ等）。
    BackendStopped { reason: BackendStopReason },

    /// An event scoped to a specific web page (tab).
    /// 特定のウェブページ（タブ）に関連するイベント
    WebPage {
        profile_id: String,
        web_page_id: WebPageId,
        event: WebPageEvent,
    },

    /// Result of listing available profiles.
    /// プロファイル一覧の取得結果。
    ProfilesListed { profiles: Vec<ProfileInfo> },

    /// Shutdown is blocked by dirty pages that require confirmation.
    /// シャットダウン要求が dirty なページによりブロックされた。
    ShutdownBlocked {
        request_id: u64,
        dirty_web_page_ids: Vec<WebPageId>,
    },

    /// Shutdown has started and is proceeding.
    /// シャットダウンが開始された。
    ShutdownProceeding { request_id: u64 },

    /// Shutdown has been cancelled.
    /// シャットダウンがキャンセルされた。
    ShutdownCancelled { request_id: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendStopReason {
    /// Stopped because an upstream shutdown was requested.
    /// 上流からの終了要求により停止した。
    ShutdownRequested,
    /// Stopped because the command channel was closed or disconnected.
    /// コマンドチャネルが閉じられた等により切断された。
    Disconnected,
    /// Stopped due to a crash or fatal termination.
    /// クラッシュ等により停止した。
    Crashed,
    /// Stopped due to an internal backend error.
    /// バックエンド内部エラーにより停止した。
    Error(BackendErrorInfo),
}

/// Events emitted from a specific web page (tab).
/// 個々のウェブページから発生するイベントを表します。
/// UIプロセス (atelier-app) はこれらのイベントを受け取り、描画や状態更新を行います。
#[derive(Debug)]
pub enum WebPageEvent {
    /// The web page was created.
    /// Webページが作成された。
    Created { request_id: u64 },

    // --- Navigation & History ---
    /// Navigation state changed (back/forward availability and loading state).
    /// ページナビゲーションの状態が変化した (戻る/進むの可否、ロード中かどうか)。
    NavigationStateChanged {
        /// Current page URL.
        /// 現在のページのURL。
        url: String,
        can_go_back: bool,
        can_go_forward: bool,
        is_loading: bool,
    },

    /// The page title was updated.
    /// ページのタイトルが更新された。
    TitleUpdated { title: String },

    /// The favicon URL was updated.
    /// ファビコンのURLが更新された。
    FaviconUrlUpdated {
        url: String, // TODO: 画像バイナリそのものが必要になる可能性もあり
    },

    // --- UI & Interaction ---
    /// The target URL display should be updated (e.g., hover on link).
    /// リンクへのマウスオーバー等により、ターゲットURLの表示を更新する必要がある。
    /// `None` の場合は表示をクリアする。
    UpdateTargetUrl { url: Option<String> },

    /// The cursor shape should be updated.
    /// カーソルの形状を変更する必要がある。
    CursorChanged { cursor_type: CursorIcon },

    /// Fullscreen state toggled.
    /// フルスクリーン状態の切り替え。
    FullscreenToggled { is_fullscreen: bool },

    // --- Window & Tab Lifecycle ---
    /// A new web page was requested (e.g., window.open, target="_blank").
    /// 新しい WebPage の作成が要求された (window.open, target="_blank")。
    NewWebPageRequested {
        target_url: String,
        // TODO: WindowOpenDisposition (Popup, NewTab, etc.) を追加
        is_popup: bool,
    },

    /// A tab close was requested (e.g., window.close).
    /// タブのクローズが要求された (window.close)。
    CloseRequested,

    /// The web page was closed.
    /// Webページがクローズされた。
    Closed,

    /// The rendering surface handle was updated.
    /// サーフェスのハンドルが更新された。
    SurfaceHandleUpdated { handle: SurfaceHandle },

    /// IME bounds information was updated.
    /// IME の候補位置などを更新する必要がある。
    ImeBoundsUpdated { update: ImeBoundsUpdate },

    /// A context menu display was requested.
    /// コンテキストメニューの表示が要求された。
    ContextMenuRequested { menu: ContextMenu },

    // --- Dialogs & Permissions (Response Required) ---
    /// A JavaScript dialog (alert/confirm/prompt) was requested.
    /// JavaScript ダイアログ (alert, confirm, prompt) の表示要求。
    /// UI側はダイアログを表示し、対応するコマンドで応答する必要がある。
    JavaScriptDialogRequested {
        request_id: u64,
        message: String,
        default_prompt_text: Option<String>,
        r#type: DialogType,
        beforeunload_reason: Option<BeforeUnloadReason>,
    },

    /// A permission request (camera, microphone, etc.).
    /// パーミッション要求 (カメラ、マイクなど)。
    PermissionRequested {
        permission: PermissionType,
        request_id: u64,
        response_channel: oneshot::Sender<bool>, // true = allow, false = block
    },

    // --- Process Lifecycle ---
    /// The renderer process exited or crashed.
    /// レンダラープロセスが消失した (クラッシュなど)。
    RenderProcessGone { crashed: bool },

    // --- Audio ---
    /// The audio playback state changed.
    /// 音声再生状態の変化。
    AudioStateChanged { is_audible: bool },

    /// The DOM HTML was read for the page.
    /// DOM の HTML が読み取られた。
    DomHtmlRead { request_id: u64, html: String },

    /// Renderer requested host-owned drag start.
    DragStartRequested { request: DragStartRequest },

    // --- Studio Specific ---
    /// The text selection range changed.
    /// テキスト選択範囲が変更された。
    SelectionChanged { text: String },

    /// The scroll position changed.
    /// スクロール位置が変更された。
    ScrollPositionChanged {
        // TODO: 座標型を定義
        x: f64,
        y: f64,
    },
}

/// Types of JavaScript dialogs or beforeunload confirmations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogType {
    Alert,
    Confirm,
    Prompt,
    BeforeUnload,
}

/// Reasons for triggering a beforeunload confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeforeUnloadReason {
    Unknown,
    CloseWebPage,
    Navigate,
    Reload,
    WindowClose,
}

/// Response payload for a JavaScript dialog request.
#[derive(Debug)]
pub enum DialogResponse {
    Success {
        input: Option<String>, // promptの場合の入力値
    },
    Cancel,
}

/// Permission categories that may be requested by a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionType {
    VideoCapture,
    AudioCapture,
    Notifications,
    Geolocation,
    // 必要に応じて追加
}
