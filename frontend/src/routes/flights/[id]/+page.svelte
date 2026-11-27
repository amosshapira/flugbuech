<script lang="ts">
  import CountryFlag from '$lib/components/CountryFlag.svelte';
  import Flashes from '$lib/components/Flashes.svelte';
  import XContestSummary from '$lib/components/XContestSummary.svelte';
  import {flightName} from '$lib/flights';
  import {formatDate, formatDistance, formatDuration, formatTime} from '$lib/formatters';
  import {i18n} from '$lib/i18n';
  import {formatAltitudeValue, units} from '$lib/units';
  import {handleViewerLinkClick, viewer} from '$lib/viewer';

  import type {Data} from './+page';

  // Compile-time feature flag: the "view track" button is only included in
  // builds with the igcviewer integration enabled (VITE_FEATURE_IGCVIEWER=1).
  const igcviewerEnabled = import.meta.env.VITE_FEATURE_IGCVIEWER === '1';

  export let data: Data;

  $: flight = data.flight;
  $: launchAt = data.flight.launchAt;
  $: landingAt = data.flight.landingAt;
  $: trackUrl = `/api/v1/flights/${flight.id}/igc/view/?marker=${$viewer.trackMarker}&style=${$viewer.renderStyle}`;
</script>

<nav class="breadcrumb" aria-label="breadcrumbs">
  <ul>
    <li><a href="/">{$i18n.t('navigation.home')}</a></li>
    <li><a href="/flights/">{$i18n.t('navigation.flights')}</a></li>
    <li class="is-active">
      <a href="./" aria-current="page">
        {$i18n.t('flight.title--flight')}
        {#if flight.number}#{flight.number}{:else}{flight.id}{/if}
      </a>
    </li>
  </ul>
</nav>

<Flashes />

<h2 class="title is-2">
  {flightName({...flight, launchAt: launchAt?.name, landingAt: landingAt?.name}, $i18n)}
</h2>

<div class="is-flex is-justify-content-space-between mb-5">
  <div class="left">
    <a href="/flights/{flight.id}/edit/" class="button is-light">
      {$i18n.t('flight.action--edit-this-flight')}
    </a>
  </div>
  <div class="right">
    {#if flight.hasIgc && igcviewerEnabled}
      <a
        href={trackUrl}
        target={$viewer.openViewerInNewTab ? '_blank' : undefined}
        rel={$viewer.openViewerInNewTab ? 'noopener' : undefined}
        on:click={(event) => handleViewerLinkClick(event, trackUrl, $viewer)}
        class="button is-light"
        data-sveltekit-reload
      >
        <span class="icon is-small"><i class="fa-solid fa-route"></i></span>&nbsp;&nbsp;{$i18n.t(
          'flights.action--view-track',
        )}
      </a>
      <a href="/api/v1/flights/{flight.id}/igc/" class="button is-light" data-sveltekit-reload>
        <span class="icon is-small"><i class="fa-solid fa-download"></i></span>&nbsp;&nbsp;{$i18n.t(
          'flights.action--download-igc',
        )}
      </a>
    {/if}
  </div>
</div>

<section>
  <table class="table is-fullwidth is-striped is-hoverable">
    <tbody>
      <tr>
        <th>
          <span class="icon is-small"><i class="fa-solid fa-list-ol"></i></span
          >&nbsp;&nbsp;{$i18n.t('flight.title--flight-number')}
        </th>
        <td>{flight.number || '-'}</td>
      </tr>
      <tr>
        <th>
          <span class="icon is-small"><i class="fa-solid fa-parachute-box"></i></span>
          &nbsp;{$i18n.t('flight.title--glider')}
        </th>
        <td>{flight.gliderName || '-'}</td>
      </tr>
      {#if flight.pilotInCommand || flight.secondPilot || flight.flightCapacity !== 'PIC'}
        <tr>
          <th>
            <span class="icon is-small"><i class="fa-solid fa-user-group"></i></span
            >&nbsp;&nbsp;{$i18n.t('flight.title--crew-role')}
          </th>
          <td>
            {#if flight.pilotInCommand}
              P1: {flight.pilotInCommand}
            {/if}
            {#if flight.secondPilot}
              {#if flight.pilotInCommand}&middot;{/if}
              P2: {flight.secondPilot}
            {/if}
            {#if flight.pilotInCommand || flight.secondPilot}&middot;{/if}
            {flight.flightCapacity}
          </td>
        </tr>
      {/if}
      <tr>
        <th>
          <span class="icon is-small"><i class="fa-solid fa-calendar-alt"></i></span
          >&nbsp;&nbsp;{$i18n.t('flight.title--date')}
        </th>
        <td>{flight.launchTime ? formatDate(flight.launchTime) : '-'}</td>
      </tr>
      <tr>
        <th>
          <span class="icon is-small"><i class="fa-solid fa-plane-departure"></i></span>
          &nbsp;{$i18n.t('flight.title--launch-site')}
        </th>
        <td>
          {#if launchAt}
            <CountryFlag countryCode={launchAt.countryCode} />
            <a href="/locations/{launchAt.id}/">{launchAt.name}</a>,
            {formatAltitudeValue(launchAt.elevation, $units.altitudeUnit)}
            {$units.altitudeUnit === 'm'
              ? $i18n.t('common.unit--m-asl')
              : $i18n.t('common.unit--ft-asl')}{:else}{$i18n.t(
              'flight.prose--unknown-site',
            )}{/if}{#if flight.launchTime}, {formatTime(flight.launchTime)} UTC{/if}
          {#if flight.hikeandfly}
            <i class="fa-solid fa-hiking" title="Hike &amp; Fly"></i>{/if}
        </td>
      </tr>
      <tr>
        <th
          ><span class="icon is-small"><i class="fa-solid fa-plane-arrival"></i></span
          >&nbsp;&nbsp;{$i18n.t('flight.title--landing-site')}</th
        >
        <td>
          {#if landingAt}
            <CountryFlag countryCode={landingAt.countryCode} />
            <a href="/locations/{landingAt.id}/">{landingAt.name}</a>,
            {formatAltitudeValue(landingAt.elevation, $units.altitudeUnit)}
            {$units.altitudeUnit === 'm'
              ? $i18n.t('common.unit--m-asl')
              : $i18n.t('common.unit--ft-asl')}{:else}{$i18n.t(
              'flight.prose--unknown-site',
            )}{/if}{#if flight.landingTime}, {formatTime(flight.landingTime)} UTC{/if}
        </td>
      </tr>
      {#if flight.launchType || flight.releaseAltitudeM !== undefined || flight.towHeightM !== undefined || flight.tugRegistration || flight.isOutlanding}
        <tr>
          <th>
            <span class="icon is-small"><i class="fa-solid fa-angle-double-up"></i></span
            >&nbsp;&nbsp;{$i18n.t('flight.title--launch-type')}
          </th>
          <td>
            {flight.launchType || '-'}
            {#if flight.releaseAltitudeM !== undefined}
              &middot; {$i18n.t('flight.prose--release-at', {
                altitude: formatAltitudeValue(flight.releaseAltitudeM, $units.altitudeUnit),
                unit:
                  $units.altitudeUnit === 'm'
                    ? $i18n.t('common.unit--m-asl')
                    : $i18n.t('common.unit--ft-asl'),
              })}
            {/if}
            {#if flight.towHeightM !== undefined}
              &middot; {$i18n.t('flight.prose--tow-height', {
                height: formatAltitudeValue(flight.towHeightM, $units.altitudeUnit),
                unit:
                  $units.altitudeUnit === 'm'
                    ? $i18n.t('common.unit--m')
                    : $i18n.t('common.unit--ft'),
              })}
            {/if}
            {#if flight.tugRegistration}
              &middot; {$i18n.t('flight.prose--tug', {registration: flight.tugRegistration})}
            {/if}
            {#if flight.isOutlanding}
              &middot;
              <span class="tag is-warning">{$i18n.t('flight.title--outlanding')}</span>
            {/if}
          </td>
        </tr>
      {/if}
      <tr>
        <th>
          <span class="icon is-small"><i class="fa-solid fa-clock"></i></span>
          &nbsp;{$i18n.t('flight.title--duration')}
        </th>
        <td>
          {flight.durationSeconds ? formatDuration(flight.durationSeconds) : '-'}
        </td>
      </tr>
      <tr>
        <th>
          <span class="icon is-small"><i class="fa-solid fa-ruler"></i></span>&nbsp;&nbsp;{$i18n.t(
            'flight.title--gps-track-distance',
          )}
        </th>
        <td>
          {flight.trackDistance
            ? formatDistance(flight.trackDistance, $units.distanceUnit)
            : '-'}</td
        >
      </tr>
      {#if flight.maxAltitudeM !== undefined || flight.heightGainM !== undefined || flight.engineRunTimeSeconds !== undefined}
        <tr>
          <th>
            <span class="icon is-small"><i class="fa-solid fa-mountain-sun"></i></span
            >&nbsp;&nbsp;{$i18n.t('flight.title--soaring-metrics')}
          </th>
          <td>
            {#if flight.maxAltitudeM !== undefined}
              {$i18n.t('flight.prose--max-altitude', {
                altitude: formatAltitudeValue(flight.maxAltitudeM, $units.altitudeUnit),
                unit:
                  $units.altitudeUnit === 'm'
                    ? $i18n.t('common.unit--m-asl')
                    : $i18n.t('common.unit--ft-asl'),
              })}
            {/if}
            {#if flight.heightGainM !== undefined}
              {#if flight.maxAltitudeM !== undefined}&middot;{/if}
              {$i18n.t('flight.prose--height-gain', {
                gain: formatAltitudeValue(flight.heightGainM, $units.altitudeUnit),
                unit:
                  $units.altitudeUnit === 'm'
                    ? $i18n.t('common.unit--m')
                    : $i18n.t('common.unit--ft'),
              })}
            {/if}
            {#if flight.engineRunTimeSeconds !== undefined}
              {#if flight.maxAltitudeM !== undefined || flight.heightGainM !== undefined}&middot;{/if}
              {$i18n.t('flight.prose--engine-run-time', {
                duration: formatDuration(flight.engineRunTimeSeconds),
              })}
            {/if}
          </td>
        </tr>
      {/if}
      <tr>
        <th>
          <span class="icon is-small"><i class="fa-solid fa-globe-americas"></i></span
          >&nbsp;&nbsp;{$i18n.t('flight.title--xcontest')}
        </th>
        <td>
          <XContestSummary
            tracktype={flight.xcontestTracktype}
            distance={flight.xcontestDistance}
            url={flight.xcontestUrl}
          />
        </td>
      </tr>
      <tr>
        <th>
          <span class="icon is-small"><i class="fa-solid fa-comment"></i></span
          >&nbsp;&nbsp;{$i18n.t('flight.title--comment')}
        </th>
        <td class="preserve-newlines">
          {flight.comment || '-'}
        </td>
      </tr>
      <tr>
        <th>
          <span class="icon is-small"><i class="fa-solid fa-film"></i></span>&nbsp;&nbsp;{$i18n.t(
            'flight.title--video-url',
          )}
        </th>
        <td>
          {#if flight.videoUrl}<a href={flight.videoUrl}>{flight.videoUrl}</a>{:else}-{/if}
        </td>
      </tr>
    </tbody>
  </table>
</section>

<style>
  table th {
    white-space: nowrap;
  }
</style>
