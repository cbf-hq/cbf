# Drag Data Field Mapping in CBF/Chromium

## 1. Scope

This note summarizes current findings about drag-and-drop data fields across:

- Chromium `content::DropData`
- CBF Chromium-side `mojom::CbfDragData`
- Rust-side `DragData` (`cbf` / `cbf-sys`)

It focuses on field semantics, directionality (browser->external vs external->browser), and API design implications for CBF.

## 2. Current CBF Payload Shape

Today `CbfProfileService::NotifyDragStartRequested` maps the following fields from Chromium `DropData` into CBF:

- `text`
- `html`
- `html_base_url`
- `url_infos`
- `filenames`
- `file_mime_types`
- `custom_data`

plus drag image and session metadata (`session_id`, `allowed_operations`, etc.).

Key code:

- `chromium/src/chrome/browser/cbf/cbf_profile_service.cc`
- `chromium/src/chrome/browser/cbf/mojom/cbf_browser.mojom`

## 3. `Atelier` Fallback in macOS Host Drag

In `BrowserViewMac::start_native_drag_session`, CBF picks a string writer in this order:

1. `request.data.text`
2. first URL in `request.data.url_infos`
3. `request.source_origin`
4. hardcoded `"Atelier"` fallback

This value is used as pasteboard writer content for `NSDraggingItem`.

Key code:

- `crates/cbf/src/platform/macos/browser_view.rs`

Practical implication: `"Atelier"` is a legacy fallback string and not core protocol data.

## 4. Chromium `DropData` Has More Fields Than CBF Exposes

`content::DropData` includes many fields beyond current CBF mapping, such as:

- file-related: `filenames`, `file_mime_types`, `file_system_files`, `filesystem_id`
- download-related: `download_metadata`, `referrer_policy`
- binary contents: `file_contents`, `file_contents_source_url`, etc.
- misc: `custom_data`, `operation`, `document_is_handling_drag`, and internal flags

Key code:

- `chromium/src/content/public/common/drop_data.h`

## 5. Directionality: Which Fields Are Used When Dragging Into Blink

For external app -> browser path, Chromium uses `DropDataToDragData`.

Used there:

- `text`
- `url_infos`
- `html` / `html_base_url`
- `filenames`
- `file_system_files`
- `file_contents*` (when `file_contents_source_url` is valid)
- `custom_data`
- `filesystem_id` (if non-empty)
- `document_is_handling_drag` (mapped to `force_default_action`)
- `referrer_policy`

Explicitly unused there:

- `download_metadata`
- `file_contents_content_disposition`

Key code:

- `chromium/src/content/browser/renderer_host/data_transfer_util.cc`

## 6. What `download_metadata` Actually Means

`download_metadata` is parsed as:

- MIME type
- file name
- URL

and is used to support file-oriented drag-out behavior (including promised/downloaded file materialization), not as a generic drag-in field.

Key code:

- `chromium/src/content/browser/download/drag_download_util.h`
- `chromium/src/content/browser/download/drag_download_util.cc`
- `chromium/src/content/browser/web_contents/web_contents_view_aura.cc`
- `chromium/src/content/app_shim_remote_cocoa/web_drag_source_mac.mm`

Operationally:

- Browser -> external app: browser side prepares/provides downloadable file semantics.
- External app -> browser: `download_metadata` is not part of normal Blink ingestion path.

## 7. CBF API Layering Guidance

Given CBF architecture constraints (browser-generic API, Chromium internals hidden behind `cbf-sys`):

- Good browser-generic expansion candidates:
  - `filenames`
  - `file_mime_types`
  - `custom_data`
  - possibly `download_metadata` (if semantics are documented clearly)
- Needs careful abstraction:
  - `referrer_policy` (map to CBF enum, do not expose Chromium mojom type)
  - `file_contents*` (size/lifetime/safety policy required)
- Chromium-specific and best kept internal:
  - `filesystem_id`
  - `file_system_files` raw semantics
  - routing and privilege flags (`view_id`, renderer/privileged markers)

Recommended rollout is additive, with empty/optional defaults for missing data.

## 8. Field Classification (Expanded)

This section incorporates the temporary memo classification and aligns it with the guidance above.

### 8.1 Browser-generic core (already in CBF today)

- `text`
- `html`
- `html_base_url`
- `url_infos`

### 8.2 Browser-generic expansion candidates

- `filenames`
- `file_mime_types`
- `custom_data` (`String -> String` style map is the safest default shape)
- `download_metadata` (only if format and direction-specific semantics are documented)

### 8.3 Needs explicit boundary design before exposure

- `referrer_policy` (map to CBF enum, avoid Chromium type leakage)
- `file_contents`
- `file_contents_image_accessible`
- `file_contents_source_url`
- `file_contents_filename_extension`
- `file_contents_content_disposition`
- `operation` (responsibility split vs existing `allowed_operations`)

For this group, define size limits, memory/lifetime policy, and normalization rules before public exposure.

### 8.4 Chromium-specific (do not expose directly in browser-generic API)

- `filesystem_id`
- `file_system_files` (raw Chromium filesystem semantics)
- `view_id`
- `did_originate_from_renderer`
- `is_from_privileged`
- `document_is_handling_drag`

These fields are tightly coupled to Chromium internals, routing, or privilege/security model.

## 9. Operational Notes

- Keep rollout additive.
- Represent unavailable values as `None`, empty list, or empty string based on field shape.
- Treat `file_contents*` as a separate phase gated on safety policy.

## 10. Current API Decision (Task 04)

The current layered policy is implemented as follows:

- `cbf::data::drag::DragData` keeps browser-generic fields only.
- `allowed_operations` is represented as browser-generic `DragOperations` in `cbf`.
- FFI/wire conversion maps `DragOperations <-> u32` in backend layers.
- `filenames`, `file_mime_types`, and `custom_data` are mapped in both directions:
  - browser -> host (`DragStartRequested`)
  - host pasteboard conversion -> browser (`convert_nspasteboard_to_drag_data`)
- Missing values are represented as empty collections.
- Chromium-internal fields (`filesystem_id`, `file_system_files`, privilege/routing flags) remain out of the browser-generic API.
