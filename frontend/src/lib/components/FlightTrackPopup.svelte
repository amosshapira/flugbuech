<script lang="ts">
  import {Map, Marker} from 'maplibre-gl';
  import {onDestroy, tick} from 'svelte';

  import {i18n} from '$lib/i18n';

  import {MAPBOX_ACCESS_TOKEN, MAPBOX_STYLE_DEFAULT, MARKER_COLOR} from './map';
  export let coordinates: [number, number][] | undefined = undefined;
  export let top = 0;
  export let left = 0;

  /** MapLibre event handlers, forwarded from the popup container. */
  export let onMouseEnter: (event: MouseEvent) => void = () => {};
  export let onMouseLeave: (event: MouseEvent) => void = () => {};

  // Track line color: primary blue
  const LINE_COLOR = MARKER_COLOR;

  // Start/end marker colors (Bulma success / danger)
  const START_MARKER_COLOR = '#48c774';
  const END_MARKER_COLOR = '#f14668';

  let container: HTMLElement | undefined;
  let map: Map | null = null;

  $: if (coordinates !== undefined) {
    void renderMap(coordinates);
  }

  /**
   * Create the map and render the track on it.
   *
   * Note: Because of the `{#key}` block in the consumer, this component is
   * re-created for every opened popup, so the map only needs to be set up
   * once.
   */
  async function renderMap(track: [number, number][]) {
    await tick(); // Ensure the container element is mounted
    if (map !== null || container === undefined) {
      return;
    }

    // The popup is a small preview, mouse events should not be captured
    const createdMap = new Map({
      container,
      style: `mapbox://styles/mapbox/${MAPBOX_STYLE_DEFAULT}`,
      accessToken: MAPBOX_ACCESS_TOKEN,
      interactive: false,
      attributionControl: false,
    });
    map = createdMap;
    createdMap.on('load', () => {
      // Add track line
      createdMap.addSource('track', {
        type: 'geojson',
        data: {
          type: 'Feature',
          geometry: {type: 'LineString', coordinates: track},
          properties: {},
        },
      });
      createdMap.addLayer({
        id: 'track-line',
        type: 'line',
        source: 'track',
        layout: {'line-join': 'round', 'line-cap': 'round'},
        paint: {'line-color': LINE_COLOR, 'line-width': 2},
      });

      // Add start and end markers
      new Marker({color: START_MARKER_COLOR}).setLngLat(track[0]).addTo(createdMap);
      new Marker({color: END_MARKER_COLOR}).setLngLat(track[track.length - 1]).addTo(createdMap);

      // Fit the map to the track bounds
      const longitudes = track.map((coord) => coord[0]);
      const latitudes = track.map((coord) => coord[1]);
      createdMap.fitBounds(
        [
          [Math.min(...longitudes), Math.min(...latitudes)], // SW
          [Math.max(...longitudes), Math.max(...latitudes)], // NE
        ],
        {padding: 20, maxZoom: 14, duration: 0},
      );
    });
  }

  onDestroy(() => {
    map?.remove();
  });
</script>

<div
  class="track-popup"
  bind:this={container}
  style:top="{top}px"
  style:left="{left}px"
  role="presentation"
  on:mouseenter={onMouseEnter}
  on:mouseleave={onMouseLeave}
>
  {#if coordinates === undefined}
    <div class="track-popup-loading">{$i18n.t('flights.prose--track-loading')}</div>
  {/if}
</div>

<style>
  .track-popup {
    position: fixed;
    z-index: 100;
    /* Keep in sync with FLIGHT_TRACK_POPUP_WIDTH / _HEIGHT in map.ts */
    width: 280px;
    height: 180px;
    background-color: white;
    border: 1px solid rgba(0, 0, 0, 0.2);
    border-radius: 4px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
    overflow: hidden;
  }

  .track-popup-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #7a7a7a;
  }
</style>
