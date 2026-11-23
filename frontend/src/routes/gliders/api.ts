import {error} from '@sveltejs/kit';
import {z} from 'zod';

import type {SvelteKitFetch} from '$lib';
import {extractResponseError} from '$lib/api';
import {AuthenticationError, ensureClientOrServerErrorCode} from '$lib/errors';

const SCHEMA_API_GLIDER = z.object({
    id: z.number(),
    manufacturer: z.string(),
    model: z.string(),
    since: z.string().optional(),
    until: z.string().optional(),
    source: z.string().optional(),
    cost: z.number().optional(),
    comment: z.string().optional(),
    registration: z.string().optional(),
    competitionId: z.string().optional(),
    isTwoSeater: z.boolean().default(false),
    faiClass: z.string().optional(),
    isMotorglider: z.boolean().default(false),
    stats: z.object({
        flights: z.number(),
        seconds: z.number(),
        secondsComplete: z.boolean(),
    }),
});

export type Glider = z.infer<typeof SCHEMA_API_GLIDER>;

/** FAI sailplane classes supported by the glider form. */
export const FAI_CLASSES = ['Club', 'Standard', '15m', '18m', '20m', 'Open', 'Two-Seater'] as const;

export type FaiClass = (typeof FAI_CLASSES)[number];

const SCHEMA_API_GLIDERS_RESPONSE = z.object({
    gliders: z.array(SCHEMA_API_GLIDER),
    lastGliderId: z.number().optional(),
});

export interface GlidersInfo {
    readonly gliders: Glider[];
    readonly lastGliderId?: number;
}

/**
 * Load gliders from API.
 */
export async function loadApiGliders(fetch: SvelteKitFetch): Promise<GlidersInfo> {
    const res = await fetch('/api/v1/gliders');
    switch (res.status) {
        case 200: {
            const response = SCHEMA_API_GLIDERS_RESPONSE.parse(await res.json());
            return {
                gliders: response.gliders,
                lastGliderId: response.lastGliderId,
            };
        }
        case 401:
            throw AuthenticationError.redirectToLogin(`/gliders/`);
        default: {
            throw error(
                ensureClientOrServerErrorCode(res.status),
                `Could not fetch gliders from API: ${await extractResponseError(res)}`,
            );
        }
    }
}
