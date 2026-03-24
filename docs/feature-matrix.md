# Feature Matrix

This document provides a high-level overview of the features supported by CBF, categorized by functionality. The status of each feature is indicated to help users and developers understand the current capabilities and limitations of CBF.

## Legend

| Symbol | Meaning |
| --- | --- |
| ✅ | Fully supported |
| 🚧 | Partial — see Notes for details |
| ❌ | Not yet implemented |

## Page Lifecycle & Navigation

| Feature | Status | Platform | Notes |
| --- | --- | --- | --- |
| Open webpage | ✅ | macOS | |
| Navigate webpage | ✅ | macOS | |
| Close webpage | ✅ | macOS | |
| Go back/forward | ✅ | macOS | |
| Reload webpage | ✅ | macOS | |
| beforeunload events | ✅ | macOS | |
| Shutdown | ✅ | macOS | |

## Surface & Input

| Feature | Status | Platform | Notes |
| --- | --- | --- | --- |
| Surface creation | ✅ | macOS | Uses `CAContextID` |
| Change surface bounds | ✅ | macOS | |
| Send mouse/key events | ✅ | macOS | |
| Send IME events | ✅ | macOS | |

## Content & Interaction

| Feature | Status | Platform | Notes |
| --- | --- | --- | --- |
| Get DOM html | ✅ | macOS | |
| Drag and Drop on webpage | ✅ | macOS | |
| Drag and Drop from other apps | ✅ | | |
| Context menu events | 🚧 | macOS | Some native items are not yet supported |

## Downloads & Print

| Feature | Status | Platform | Notes |
| --- | --- | --- | --- |
| Download management | ✅ | macOS | |
| Show print dialog UI | 🚧 | macOS | UI can be shown; window can't activated |
| Show print preview UI | ❌ | | |

## Profile & Extensions

| Feature | Status | Platform | Notes |
| --- | --- | --- | --- |
| Open webpage with profile | ✅ | macOS | |
| Get profile list | ✅ | macOS | |
| Get profile info | ✅ | macOS | |
| Get extension list | ✅ | macOS | |
| Extension inline UI | ✅ | macOS | |
| Full extension support | 🚧 | macOS | |

## Developer Tools & Built-in Pages

| Feature | Status | Platform | Notes |
| --- | --- | --- | --- |
| DevTools UI | 🚧 | macOS | You can embed DevTools in your app |
| `chrome://version` | ✅ | macOS | |
| `chrome://history` | 🚧 | macOS | Deleting all history at once is not yet supported |
| `chrome://settings` | 🚧 | macOS | Some settings options are not yet available |
