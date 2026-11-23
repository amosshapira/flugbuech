import {writable} from 'svelte/store';

/** Units for displaying altitudes. */
export type AltitudeUnit = 'm' | 'ft';
/** Units for displaying distances. */
export type DistanceUnit = 'km' | 'mi' | 'nm';

export const ALTITUDE_UNITS: readonly AltitudeUnit[] = ['m', 'ft'];
export const DISTANCE_UNITS: readonly DistanceUnit[] = ['km', 'mi', 'nm'];

export const DEFAULT_ALTITUDE_UNIT: AltitudeUnit = 'm';
export const DEFAULT_DISTANCE_UNIT: DistanceUnit = 'km';

const FT_PER_M = 3.28084;
const M_PER_FT = 0.3048;
const MI_PER_KM = 0.621371;
const NM_PER_KM = 0.539957;

/** Convert metres to feet. */
export function metresToFeet(metres: number): number {
    return metres * FT_PER_M;
}

/** Convert feet to metres. */
export function feetToMetres(feet: number): number {
    return feet * M_PER_FT;
}

/** Convert a distance in kilometres to the given unit. */
export function kmToUnit(km: number, unit: DistanceUnit): number {
    switch (unit) {
        case 'km':
            return km;
        case 'mi':
            return km * MI_PER_KM;
        case 'nm':
            return km * NM_PER_KM;
    }
}

/** Convert a distance in the given unit to kilometres. */
export function unitToKm(value: number, unit: DistanceUnit): number {
    switch (unit) {
        case 'km':
            return value;
        case 'mi':
            return value / MI_PER_KM;
        case 'nm':
            return value / NM_PER_KM;
    }
}

/** Convert an altitude in metres to the given unit. */
export function metresToUnit(metres: number, unit: AltitudeUnit): number {
    return unit === 'm' ? metres : metresToFeet(metres);
}

/** Convert an altitude in the given unit to metres. */
export function unitToMetres(value: number, unit: AltitudeUnit): number {
    return unit === 'm' ? value : feetToMetres(value);
}

/**
 * Format an altitude given in metres in the given unit.
 *
 * Metric altitudes are shown as whole metres, imperial altitudes as whole
 * feet.
 */
export function formatAltitudeValue(metres: number, unit: AltitudeUnit): string {
    const value = metresToUnit(metres, unit);
    return Math.round(value).toString();
}

/**
 * Format a distance given in kilometres in the given unit.
 *
 * Example: formatDistanceValue(13.37123, 'km') === "13.37"
 */
export function formatDistanceValue(km: number, unit: DistanceUnit): string {
    return parseFloat(kmToUnit(km, unit).toFixed(2)).toString();
}

// Unit preference store

interface StoredUnits {
    altitudeUnit?: string;
    distanceUnit?: string;
}

export interface UnitPrefs {
    altitudeUnit: AltitudeUnit;
    distanceUnit: DistanceUnit;
}

const STORAGE_KEY = 'flugbuech-units';

function isAltitudeUnit(value: string): value is AltitudeUnit {
    return (ALTITUDE_UNITS as readonly string[]).includes(value);
}

function isDistanceUnit(value: string): value is DistanceUnit {
    return (DISTANCE_UNITS as readonly string[]).includes(value);
}

function parseStoredUnits(raw: string | null): Partial<UnitPrefs> {
    if (!raw) {
        return {};
    }
    let parsed: StoredUnits;
    try {
        parsed = JSON.parse(raw) as StoredUnits;
    } catch {
        return {};
    }
    const prefs: Partial<UnitPrefs> = {};
    if (parsed.altitudeUnit && isAltitudeUnit(parsed.altitudeUnit)) {
        prefs.altitudeUnit = parsed.altitudeUnit;
    }
    if (parsed.distanceUnit && isDistanceUnit(parsed.distanceUnit)) {
        prefs.distanceUnit = parsed.distanceUnit;
    }
    return prefs;
}

/**
 * Store with the units used to display altitudes and distances.
 *
 * For logged-in users it reflects the preference saved on their user record.
 * For guests (or until the profile is loaded) it can be overridden per
 * browser session via sessionStorage.
 */
export const units = writable<UnitPrefs>({
    altitudeUnit: DEFAULT_ALTITUDE_UNIT,
    distanceUnit: DEFAULT_DISTANCE_UNIT,
});

/**
 * Restore the per-session unit override, if any. Called once on client-side
 * app initialization.
 */
export function initializeUnits(): void {
    if (typeof sessionStorage === 'undefined') {
        return;
    }
    const prefs = parseStoredUnits(sessionStorage.getItem(STORAGE_KEY));
    if (prefs.altitudeUnit || prefs.distanceUnit) {
        units.update((u) => ({...u, ...prefs}));
    }
}

/**
 * Override the units for the current browser session.
 *
 * For logged-in users the change should be persisted to their user record
 * instead, so only guests end up here.
 */
export function setSessionUnits(prefs: Partial<UnitPrefs>): void {
    units.update((u) => ({...u, ...prefs}));
    if (typeof sessionStorage !== 'undefined') {
        sessionStorage.setItem(STORAGE_KEY, JSON.stringify({...prefs}));
    }
}

/**
 * Clear the per-session unit override (e.g. because the user saved new
 * defaults on their profile).
 */
export function clearSessionUnits(): void {
    if (typeof sessionStorage !== 'undefined') {
        sessionStorage.removeItem(STORAGE_KEY);
    }
}

/**
 * Apply a user's saved unit preference (from their profile).
 *
 * Takes precedence over the defaults, but a per-session override wins.
 */
export function applyProfileUnits(altitudeUnit: string, distanceUnit: string): void {
    if (typeof sessionStorage !== 'undefined' && sessionStorage.getItem(STORAGE_KEY)) {
        return;
    }
    const prefs: Partial<UnitPrefs> = {};
    if (isAltitudeUnit(altitudeUnit)) {
        prefs.altitudeUnit = altitudeUnit;
    }
    if (isDistanceUnit(distanceUnit)) {
        prefs.distanceUnit = distanceUnit;
    }
    if (prefs.altitudeUnit || prefs.distanceUnit) {
        units.update((u) => ({...u, ...prefs}));
    }
}
