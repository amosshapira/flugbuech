<script lang="ts">
  import {apiDelete, extractResponseError} from '$lib/api';
  import CountryFlag from '$lib/components/CountryFlag.svelte';
  import DialogModal from '$lib/components/DialogModal.svelte';
  import Flashes from '$lib/components/Flashes.svelte';
  import MessageModal from '$lib/components/MessageModal.svelte';
  import {flightName} from '$lib/flights';
  import {formatDate, formatDistance, formatDuration} from '$lib/formatters';
  import {i18n} from '$lib/i18n';
  import {addFlash} from '$lib/stores';
  import {formatAltitudeValue, units} from '$lib/units';
  import {handleViewerLinkClick, viewer} from '$lib/viewer';

  import {goto, invalidateAll} from '$app/navigation';

  import type {Data} from './+page';
  import type {FlightListItem} from './api';
  import {apiBackfillIgc, type BackfillIgcResult, SubmitError} from './api';

  // Compile-time feature flag: the "view track" button is only included in
  // builds with the igcviewer integration enabled (VITE_FEATURE_IGCVIEWER=1).
  const igcviewerEnabled = import.meta.env.VITE_FEATURE_IGCVIEWER === '1';

  // The Co-Pilot column is only shown when at least one flight in the list
  // has a second pilot recorded (a literal "NIL" from IGC headers does not
  // count as one).
  $: hasSecondPilot = data.flights.some(
    (flight) => flight.secondPilot && flight.secondPilot.toUpperCase() !== 'NIL',
  );

  export let data: Data;

  let flashes: Flashes;

  let flightToDelete: FlightListItem | undefined;
  let deleting = false;
  let backfilling = false;
  let backfillError: {type: 'authentication'} | {type: 'api-error'; message: string} | undefined;
  let deleteError: {type: 'authentication'} | {type: 'api-error'; message: string} | undefined;

  /**
   * Re-parse the stored IGC files and back-fill empty flight fields (glider,
   * launch/landing site, crew, max altitude, height gain).
   */
  async function backfillFromIgc(): Promise<void> {
    backfilling = true;
    backfillError = undefined;
    try {
      const result: BackfillIgcResult = await apiBackfillIgc();
      addFlash({
        message: $i18n.t('flights.prose--backfill-success', {
          flights: result.flightsUpdated,
          gliders: result.glidersSet,
          locations: result.locationsSet,
          crew: result.crewSet,
          altitudes: result.altitudesSet,
        }),
        severity: result.flightsUpdated > 0 ? 'success' : 'info',
        icon: 'fa-circle-check',
      });
      await invalidateAll();
    } catch (error) {
      if (error instanceof SubmitError && error.data.type === 'authentication') {
        backfillError = {type: 'authentication'};
      } else if (error instanceof SubmitError) {
        backfillError = error.data;
      } else {
        backfillError = {type: 'api-error', message: `${error}`};
      }
    } finally {
      backfilling = false;
    }
  }

  function flightListItemName(f: FlightListItem): string {
    return flightName(
      {
        ...f,
        launchAt: f.launchAt === undefined ? undefined : data.locations[f.launchAt]?.name,
        landingAt: f.landingAt === undefined ? undefined : data.locations[f.landingAt]?.name,
      },
      $i18n,
    );
  }

  async function deleteFlight(): Promise<void> {
    if (flightToDelete !== undefined) {
      deleting = true;
      console.info(`Deleting flight with ID ${flightToDelete.id}`);

      const response = await apiDelete(`/api/v1/flights/${flightToDelete.id}/`);
      switch (response.status) {
        case 204:
          // Success
          addFlash({
            message: $i18n.t('flights.prose--delete-success', {
              flight: flightListItemName(flightToDelete),
            }),
            severity: 'success',
            icon: 'fa-trash-can',
          });
          goto('/flights/');
          break;
        case 401:
          deleteError = {type: 'authentication'};
          break;
        default: {
          deleteError = {
            type: 'api-error',
            message: await extractResponseError(response),
          };
          break;
        }
      }

      // Show flash message
      flashes.update(true);

      // Hide delete dialog
      flightToDelete = undefined;
      deleting = false;

      // Reload data
      invalidateAll();
    }
  }
</script>

{#if backfillError?.type === 'authentication'}
  <MessageModal
    type="warning"
    title={$i18n.t('common.error--authentication-error')}
    message={$i18n.t('common.error--login-session-expired')}
    showClose={false}
  >
    <section slot="buttons">
      <a href="/auth/login/" class="button is-warning">
        {$i18n.t('navigation.login')}
      </a>
    </section>
  </MessageModal>
{:else if backfillError?.type === 'api-error'}
  <MessageModal
    type="error"
    title={$i18n.t('common.error--api-error')}
    message={$i18n.t('flights.error--backfill-error', {message: backfillError.message})}
    showClose={true}
    on:closed={() => (backfillError = undefined)}
  />
{:else if deleteError?.type === 'authentication'}
  <MessageModal
    type="warning"
    title={$i18n.t('common.error--authentication-error')}
    message={$i18n.t('common.error--login-session-expired')}
    showClose={false}
  >
    <section slot="buttons">
      <a href="/auth/login/" class="button is-warning">
        {$i18n.t('navigation.login')}
      </a>
    </section>
  </MessageModal>
{:else if deleteError?.type === 'api-error'}
  <MessageModal
    type="error"
    title={$i18n.t('common.error--api-error')}
    message={$i18n.t('flights.error--delete-error', {message: deleteError.message})}
    showClose={true}
    on:closed={() => (deleteError = undefined)}
  />
{:else if flightToDelete !== undefined}
  <DialogModal
    title={$i18n.t('flights.action--delete-flight')}
    message={$i18n.t('flights.prose--delete-confirm', {flight: flightListItemName(flightToDelete)})}
    dialogClass="is-danger"
  >
    <section slot="buttons">
      <button class="button is-white" on:click={() => (flightToDelete = undefined)}>
        {$i18n.t('common.action--no-cancel')}
      </button>
      <button class="button is-danger" disabled={deleting} on:click={() => void deleteFlight()}>
        {$i18n.t('common.action--yes-delete')}
      </button>
    </section>
  </DialogModal>
{/if}

<nav class="breadcrumb" aria-label="breadcrumbs">
  <ul>
    <li><a href="/">{$i18n.t('navigation.home')}</a></li>
    <li class="is-active"><a href="./" aria-current="page">{$i18n.t('navigation.flights')}</a></li>
  </ul>
</nav>

<Flashes bind:this={flashes} />

<h2 class="title is-2">{$i18n.t('flights.title--your-flights')}</h2>

<section>
  <p class="content">
    {$i18n.t('flights.prose--flight-log-count', {count: data.flights.length})}
  </p>

  <p class="content" style="display: flex; justify-content: space-between;">
    <a href="/flights/add/" class="button is-primary">
      {$i18n.t('flights.action--add-flight')}
    </a>
    <button
      class="button is-light"
      disabled={backfilling}
      title={$i18n.t('flights.hint--backfill-igc')}
      on:click={() => void backfillFromIgc()}
    >
      {$i18n.t('flights.action--backfill-igc')}
    </button>
    <a href="/flights/import/csv/" class="button is-light"
      >{$i18n.t('flights.action--import-csv')}</a
    >
  </p>

  <div class="table-container">
    <table class="table is-fullwidth is-striped is-hoverable">
      <thead>
        <tr>
          <th>#</th>
          <th>{$i18n.t('flights.column--date')}</th>
          <th>{$i18n.t('flights.column--glider')}</th>
          {#if hasSecondPilot}
            <th title={$i18n.t('flights.column--p2-title')}>{$i18n.t('flights.column--p2')}</th>
          {/if}
          <th>{$i18n.t('flights.column--launch')}</th>
          <th>{$i18n.t('flights.column--landing')}</th>
          <th>{$i18n.t('flights.column--duration')}</th>
          <th>{$i18n.t('flights.column--track-distance')}</th>
          <th>{$i18n.t('flights.column--actions')}</th>
        </tr>
      </thead>
      <tbody>
        {#each data.flights as flight (flight.id)}
          {@const launchAt = flight.launchAt ? data.locations[flight.launchAt] : undefined}
          {@const landingAt = flight.landingAt ? data.locations[flight.landingAt] : undefined}
          {@const trackUrl = `/api/v1/flights/${flight.id}/igc/view/?marker=${$viewer.trackMarker}&style=${$viewer.renderStyle}`}
          <tr>
            <td class="no-wrap">{flight.number ?? '-'}</td>
            <td class="no-wrap">
              {#if flight.launchTime !== undefined}{formatDate(flight.launchTime)}{:else}-{/if}
            </td>
            <td class="no-wrap">{flight.gliderName ?? '-'}</td>
            {#if hasSecondPilot}
              <td class="no-wrap">
                {flight.secondPilot && flight.secondPilot.toUpperCase() !== 'NIL'
                  ? flight.secondPilot
                  : ''}
              </td>
            {/if}
            <td
              title={launchAt !== undefined
                ? `${formatAltitudeValue(launchAt.elevation, $units.altitudeUnit)} ${
                    $units.altitudeUnit === 'm'
                      ? $i18n.t('common.unit--m-asl')
                      : $i18n.t('common.unit--ft-asl')
                  }`
                : ''}
            >
              {#if launchAt}
                <CountryFlag countryCode={launchAt.countryCode} />
                <a class="subtle-link" href="/locations/{launchAt.id}">{launchAt.name}</a
                >{#if flight.hikeandfly}&nbsp;<i class="fa-solid fa-hiking" title="Hike &amp; Fly"
                  ></i>{/if}
              {:else}
                -
              {/if}
            </td>
            <td
              title={landingAt !== undefined
                ? `${formatAltitudeValue(landingAt.elevation, $units.altitudeUnit)} ${
                    $units.altitudeUnit === 'm'
                      ? $i18n.t('common.unit--m-asl')
                      : $i18n.t('common.unit--ft-asl')
                  }`
                : ''}
            >
              {#if landingAt}
                <CountryFlag countryCode={landingAt.countryCode} />
                <a class="subtle-link" href="/locations/{landingAt.id}">{landingAt.name}</a>
              {:else}
                -
              {/if}
            </td>
            <td class="no-wrap">
              {#if flight.durationSeconds}{formatDuration(flight.durationSeconds)}{:else}-{/if}
            </td>
            <td class="no-wrap">
              {#if flight.trackDistance}{formatDistance(
                  flight.trackDistance,
                  $units.distanceUnit,
                )}{:else}-{/if}
            </td>
            <td class="no-wrap">
              <a
                class="icon"
                title={$i18n.t('flights.action--view-flight')}
                aria-label={$i18n.t('flights.action--view-flight')}
                href="/flights/{flight.id}/"
              >
                <i class="fa-solid fa-eye"></i>
              </a>
              <a
                class="icon"
                title={$i18n.t('flights.action--edit-flight')}
                aria-label={$i18n.t('flights.action--edit-flight')}
                href="/flights/{flight.id}/edit/"
                data-sveltekit-preload-data="tap"
              >
                <i class="fa-solid fa-pen-square"></i>
              </a>
              <button
                class="icon has-text-danger"
                title={$i18n.t('flights.action--delete-flight')}
                aria-label={$i18n.t('flights.action--delete-flight')}
                on:click={() => (flightToDelete = flight)}
              >
                <i class="fa-solid fa-trash-alt"></i>
              </button>
              {#if flight.hasIgc && igcviewerEnabled}
                <a
                  class="icon"
                  title={$i18n.t('flights.action--view-track')}
                  aria-label={$i18n.t('flights.action--view-track')}
                  href={trackUrl}
                  target={$viewer.openViewerInNewTab ? '_blank' : undefined}
                  rel={$viewer.openViewerInNewTab ? 'noopener' : undefined}
                  on:click={(event) => handleViewerLinkClick(event, trackUrl, $viewer)}
                  data-sveltekit-reload
                >
                  <i class="fa-solid fa-route"></i>
                </a>
                <a
                  class="icon"
                  title={$i18n.t('flights.action--download-igc')}
                  aria-label={$i18n.t('flights.action--download-igc')}
                  href="/api/v1/flights/{flight.id}/igc/"
                  data-sveltekit-reload
                >
                  <i class="fa-solid fa-download"></i>
                </a>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</section>

<style>
  table button {
    background-color: transparent;
    border: none;
    cursor: pointer;
  }

  table button:hover i {
    color: #c41e1e;
  }
</style>
