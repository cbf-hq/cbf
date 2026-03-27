# cbf-compositor

`cbf-compositor` is the scene-based browser surface compositor for CBF applications.

- ownership tree: backend object relationships such as transient browsing
  contexts being owned by a parent browsing context
- composition tree: which window displays which browser surface, in what order,
  and at what bounds

Within a window, `WindowCompositionSpec.items` defines front-to-back stacking
order. The first item is topmost.

Each scene item declares a hit-test policy:

- `Passthrough`: never receives pointer hit-tests
- `Bounds`: receives pointer hit-tests across the full item bounds
- `RegionSnapshot`: receives pointer hit-tests only inside the latest pushed
  region snapshot

The compositor only manages browser-managed surfaces:

- `BrowsingContext`
- `TransientBrowsingContext`

Host-native UI remains outside of the scene graph.

The current implementation provides:

- scene/state management for `BrowsingContext` and `TransientBrowsingContext`
- Chrome event adapters for surface handles and transient popup lifecycle hints
- macOS host-window attachment through `raw-window-handle`
- scene rendering via a single `CompositorViewMac` that manages multiple
  `CALayerHost` surfaces
- input routing for key, edit action, mouse wheel, IME, and transient popup
  commands
- host-side context menu, choice menu, and native drag integration for the
  active surface target

The macOS implementation attaches exactly one compositor-owned `NSView` to the
host content view. That responder view becomes first responder and routes
keyboard, IME, mouse, wheel, menu, and drag interactions to the appropriate
scene item based on composition order and hit-testing. Region snapshots use
item-local CSS-pixel rectangles with a top-left origin.
