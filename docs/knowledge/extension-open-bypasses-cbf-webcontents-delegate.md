# Extension Open Paths That Bypass CBF WebContentsDelegate

## Scope

This note captures why extension-triggered page opens can still create and focus
regular Chromium windows even after introducing
`BrowsingContextOpenRequested` / `RespondBrowsingContextOpen`.

## Confirmed Behavior

`BrowsingContextOpenRequested` is emitted only for paths that pass through
CBF-owned `WebContentsDelegate`:

- `chrome/browser/cbf/cbf_web_contents_delegate.cc`
- `chrome/browser/cbf/cbf_profile_service.cc`

So renderer-driven `window.open` / target-disposition navigations from CBF-owned
web contents are host-mediated.

However, several extension APIs open tabs/windows through Chrome's browser window
abstractions directly, which can create/show a normal Chromium window when no
Chrome browser window exists.

## Main Bypass Paths

### 1. `chrome.runtime.openOptionsPage()`

`ExtensionTabUtil::OpenOptionsPageFromAPI()` falls back to creating a browser
window when no browser is available, then opens options UI there.

File:

- `chrome/browser/extensions/extension_tab_util.cc`

### 2. Runtime delegate URL opens

`ChromeRuntimeAPIDelegate::OpenURL()` uses `FindLastActiveWithProfile()` and
creates a `Browser` when missing, then navigates in a foreground tab.

File:

- `chrome/browser/extensions/api/runtime/chrome_runtime_api_delegate.cc`

### 3. Extension tab helper fallback

`OpenTabHelper::FindOrCreateBrowser()` can create/show a browser window in its
`create_if_needed` fallback.

File:

- `chrome/browser/extensions/open_tab_helper.cc`

## Why This Caused the Reported Issue

After extension install, onboarding flows often call extension APIs such as
`runtime.openOptionsPage()` (or related open-tab APIs). Those do not require a
CBF-managed source `WebContents` delegate path, so they can bypass CBF
`BrowsingContextOpenRequested` and create/focus a regular Chromium window.

## Practical Mitigation

When `cbf::CbfService::GetForProcess()` is active:

1. Detect extension open attempts before browser-window creation.
2. Route them to `CbfProfileService::NotifyNewWebPageRequested(...)` with
   `source = nullptr` and an appropriate `WindowOpenDisposition`.
3. Skip browser-window creation/show in those fallback branches.

This keeps window ownership host-mediated and prevents surprise Chromium window
focus changes in CBF apps.
