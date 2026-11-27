import {writable} from 'svelte/store';

export const DEFAULT_OPEN_VIEWER_IN_NEW_TAB = true;

export const DEFAULT_REUSE_VIEWER_TAB = true;

/** Window name used for the viewer tab, so an already-open tab can be reused. */
export const VIEWER_WINDOW_NAME = 'igcviewer';

/** Replay marker for the IGC track viewer. */
export type TrackMarker = 'glider' | 'paraglider';

/** IGC track viewer rendering style. */
export type RenderStyle = 'ground' | 'deck';

export const DEFAULT_TRACK_MARKER: TrackMarker = 'glider';
export const DEFAULT_RENDER_STYLE: RenderStyle = 'ground';
export const TRACK_MARKERS: readonly TrackMarker[] = ['glider', 'paraglider'];
export const RENDER_STYLES: readonly RenderStyle[] = ['ground', 'deck'];

/** How the IGC track viewer is opened. */
export interface ViewerPrefs {
    openViewerInNewTab: boolean;
    reuseViewerTab: boolean;
    trackMarker: TrackMarker;
    renderStyle: RenderStyle;
}

const STORAGE_KEY = 'flugbuech-viewer';

/**
 * Store with the preferences for the IGC track viewer.
 *
 * Mirrors the preferences saved on the user record; persisted per browser so
 * they are available on flight pages without loading the profile first.
 */
export const viewer = writable<ViewerPrefs>({
    openViewerInNewTab: DEFAULT_OPEN_VIEWER_IN_NEW_TAB,
    reuseViewerTab: DEFAULT_REUSE_VIEWER_TAB,
    trackMarker: DEFAULT_TRACK_MARKER,
    renderStyle: DEFAULT_RENDER_STYLE,
});

/**
 * Restore the stored viewer preferences, if any. Called once on client-side
 * app initialization.
 */
export function initializeViewer(): void {
    if (typeof localStorage === 'undefined') {
        return;
    }
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw === null) {
        return;
    }
    let parsed: Partial<ViewerPrefs>;
    try {
        parsed = JSON.parse(raw) as Partial<ViewerPrefs>;
    } catch {
        return;
    }
    if (typeof parsed.openViewerInNewTab === 'boolean') {
        viewer.update((v) => ({...v, openViewerInNewTab: parsed.openViewerInNewTab as boolean}));
    }
    if (typeof parsed.reuseViewerTab === 'boolean') {
        viewer.update((v) => ({...v, reuseViewerTab: parsed.reuseViewerTab as boolean}));
    }
    if (parsed.trackMarker && (TRACK_MARKERS as readonly string[]).includes(parsed.trackMarker)) {
        viewer.update((v) => ({...v, trackMarker: parsed.trackMarker as TrackMarker}));
    }
    if (parsed.renderStyle && (RENDER_STYLES as readonly string[]).includes(parsed.renderStyle)) {
        viewer.update((v) => ({...v, renderStyle: parsed.renderStyle as RenderStyle}));
    }
}

/** A partial viewer preferences object; `undefined` values are ignored. */
export type ViewerPrefsUpdate = Partial<ViewerPrefs>;

/**
 * Apply the user's saved viewer preferences (from their profile) and persist
 * them for this browser.
 *
 * Only the provided fields are changed; fields set to `undefined` keep their
 * current value.
 */
export function applyProfileViewer(update: ViewerPrefsUpdate): void {
    const defined = Object.fromEntries(
        Object.entries(update).filter(([, value]) => value !== undefined),
    ) as Partial<ViewerPrefs>;
    viewer.update((v) => ({...v, ...defined}));
    if (typeof localStorage !== 'undefined') {
        const stored = JSON.parse(
            localStorage.getItem(STORAGE_KEY) ?? '{}',
        ) as Partial<ViewerPrefs>;
        localStorage.setItem(STORAGE_KEY, JSON.stringify({...stored, ...defined}));
    }
}

/**
 * Click handler for "View track" links: when the viewer opens in a separate
 * tab and tab reuse is enabled, open it with a fixed window name so the
 * browser navigates the already-open viewer tab to the new flight instead of
 * opening another tab each time. Otherwise the default link behaviour applies
 * (same tab, or a separate tab via target="_blank").
 */
export function handleViewerLinkClick(event: MouseEvent, url: string, prefs: ViewerPrefs): void {
    if (prefs.openViewerInNewTab && prefs.reuseViewerTab) {
        event.preventDefault();
        window.open(url, VIEWER_WINDOW_NAME);
    }
}
