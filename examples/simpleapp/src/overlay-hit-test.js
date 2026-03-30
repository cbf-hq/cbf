(function () {
  const DEFAULT_SELECTOR = '[data-cbf-hit-test-region]';
  const DEFAULT_MODE = 'consume-listed-regions';

  function normalizeRoot(root) {
    return root && typeof root.querySelectorAll === 'function' ? root : document;
  }

  function rootOffset(root) {
    if (!root || root === document || root === document.documentElement) {
      return { x: 0, y: 0 };
    }
    if (root instanceof Element) {
      const rect = root.getBoundingClientRect();
      return { x: rect.left, y: rect.top };
    }
    return { x: 0, y: 0 };
  }

  function rectEquals(a, b) {
    return a.x === b.x && a.y === b.y && a.width === b.width && a.height === b.height;
  }

  function snapshotEquals(a, b) {
    if (!a || !b) return false;
    if (a.coordinateSpace !== b.coordinateSpace) return false;
    if (a.mode !== b.mode) return false;
    if (a.regions.length !== b.regions.length) return false;
    for (let i = 0; i < a.regions.length; i += 1) {
      if (!rectEquals(a.regions[i], b.regions[i])) return false;
    }
    return true;
  }

  function collectRegions(root, selector) {
    const offset = rootOffset(root);
    const regions = [];
    const elements = root.querySelectorAll(selector);
    for (const element of elements) {
      const rects = element.getClientRects();
      for (const rect of rects) {
        if (rect.width <= 0 || rect.height <= 0) continue;
        regions.push({
          x: rect.left - offset.x,
          y: rect.top - offset.y,
          width: rect.width,
          height: rect.height,
        });
      }
    }
    return regions;
  }

  class OverlayHitTestController {
    constructor() {
      this.disconnect();
      this.snapshotId = 0;
      this.lastSnapshot = null;
    }

    install(options) {
      this.disconnect();
      const settings = options || {};
      if (typeof settings.sendSnapshot !== 'function') {
        throw new Error('sendSnapshot is required');
      }

      this.sendSnapshot = settings.sendSnapshot;
      this.root = normalizeRoot(settings.root);
      this.selector = settings.selector || DEFAULT_SELECTOR;
      this.mode = settings.mode || DEFAULT_MODE;
      this.pendingFrame = null;

      this.mutationObserver = new MutationObserver(() => this.scheduleFlush());
      this.mutationObserver.observe(this.root, {
        attributes: true,
        childList: true,
        subtree: true,
      });

      this.resizeObserver = new ResizeObserver(() => this.scheduleFlush());
      const observeRoot = this.root === document ? document.documentElement : this.root;
      this.resizeObserver.observe(observeRoot);
      for (const element of this.root.querySelectorAll(this.selector)) {
        this.resizeObserver.observe(element);
      }

      this.onWindowResize = () => this.scheduleFlush();
      this.onScroll = () => this.scheduleFlush();
      window.addEventListener('resize', this.onWindowResize);
      window.addEventListener('scroll', this.onScroll, true);

      this.flush();
    }

    disconnect() {
      if (this.pendingFrame !== null) {
        window.cancelAnimationFrame(this.pendingFrame);
      }
      this.pendingFrame = null;
      if (this.mutationObserver) this.mutationObserver.disconnect();
      if (this.resizeObserver) this.resizeObserver.disconnect();
      if (this.onWindowResize) window.removeEventListener('resize', this.onWindowResize);
      if (this.onScroll) window.removeEventListener('scroll', this.onScroll, true);
      this.sendSnapshot = null;
      this.root = document;
      this.selector = DEFAULT_SELECTOR;
      this.mode = DEFAULT_MODE;
      this.mutationObserver = null;
      this.resizeObserver = null;
      this.onWindowResize = null;
      this.onScroll = null;
    }

    scheduleFlush() {
      if (this.pendingFrame !== null) return;
      this.pendingFrame = window.requestAnimationFrame(() => {
        this.pendingFrame = null;
        this.flush();
      });
    }

    flush() {
      if (!this.sendSnapshot) return;
      if (this.resizeObserver) {
        for (const element of this.root.querySelectorAll(this.selector)) {
          this.resizeObserver.observe(element);
        }
      }
      const snapshot = {
        snapshotId: ++this.snapshotId,
        coordinateSpace: 'item-local-css-px',
        mode: this.mode,
        regions: collectRegions(this.root, this.selector),
      };
      if (snapshotEquals(this.lastSnapshot, snapshot)) return;
      this.lastSnapshot = {
        coordinateSpace: snapshot.coordinateSpace,
        mode: snapshot.mode,
        regions: snapshot.regions.map((region) => ({ ...region })),
      };
      this.sendSnapshot(snapshot);
    }
  }

  const controller = new OverlayHitTestController();
  window.CbfOverlayHitTest = {
    install(options) {
      controller.install(options);
    },
    disconnect() {
      controller.disconnect();
    },
    flush() {
      controller.flush();
    },
  };
})();
