import {describe, it, expect} from 'vitest';

import {formatDistance, formatDuration} from './formatters';
import {kmToUnit, metresToUnit, unitToKm, unitToMetres} from './units';

describe('formatDuration', () => {
    const testCases = [
        [0, '00:00'],
        [60 * 7, '00:07'],
        [60 * 59, '00:59'],
        [60 * 60, '01:00'],
        [60 * 61, '01:01'],
        [60 * 642, '10:42'],
        [3600 * 99 + 3599, '99:59'],
        [3600 * 99 + 3600, '100:00'],
    ] as const;

    for (const [seconds, expected] of testCases) {
        it(`formats ${seconds} seconds`, () => {
            expect(formatDuration(seconds)).to.equal(expected);
        });
    }
});

describe('formatDistance', () => {
    const testCases = [
        [0, '0 km'],
        [3, '3 km'],
        [3.1, '3.1 km'],
        [3.45, '3.45 km'],
        [3.4521, '3.45 km'],
    ] as const;

    for (const [km, expected] of testCases) {
        it(`formats ${km} km`, () => {
            expect(formatDistance(km)).to.equal(expected);
        });
    }

    const unitTestCases = [
        [0, 'km', '0 km'],
        [3, 'mi', '1.86 mi'],
        [100, 'km', '100 km'],
        [100, 'mi', '62.14 mi'],
        [100, 'nm', '54 nm'],
        [12.34, 'nm', '6.66 nm'],
    ] as const;

    for (const [km, unit, expected] of unitTestCases) {
        it(`formats ${km} km as ${expected}`, () => {
            expect(formatDistance(km, unit)).to.equal(expected);
        });
    }
});

describe('unit conversion', () => {
    it('round-trips kilometres', () => {
        for (const unit of ['km', 'mi', 'nm'] as const) {
            expect(unitToKm(kmToUnit(12.3456, unit), unit)).to.be.closeTo(12.3456, 0.0001);
        }
    });

    it('round-trips metres', () => {
        for (const unit of ['m', 'ft'] as const) {
            expect(unitToMetres(metresToUnit(1234.5678, unit), unit)).to.be.closeTo(
                1234.5678,
                0.0001,
            );
        }
    });
});
