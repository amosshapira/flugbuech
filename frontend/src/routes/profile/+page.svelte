<script lang="ts">
  import Flashes from '$lib/components/Flashes.svelte';
  import MessageModal from '$lib/components/MessageModal.svelte';
  import SubstitutableText from '$lib/components/SubstitutableText.svelte';
  import {i18n} from '$lib/i18n';
  import type {AltitudeUnit, DistanceUnit} from '$lib/units';
  import {ALTITUDE_UNITS, DISTANCE_UNITS, clearSessionUnits, units} from '$lib/units';
  import {applyProfileViewer} from '$lib/viewer';

  import {invalidateAll} from '$app/navigation';

  import type {Data} from './+page';
  import {apiUpdateProfile} from './api';

  export let data: Data;

  let flashes: Flashes;

  // Error handling
  let errorModal: {type: 'api-error'; message: string} | undefined;

  // Compile-time feature flag: the track viewer setting is only shown in
  // builds with the igcviewer integration enabled.
  const igcviewerEnabled = import.meta.env.VITE_FEATURE_IGCVIEWER === '1';

  function updateOpenViewerInNewTab(openInNewTab: boolean): void {
    apiUpdateProfile({openViewerInNewTab: openInNewTab})
      .then((result) => {
        if (result.success) {
          applyProfileViewer(openInNewTab);
          invalidateAll();
        } else {
          errorModal = {
            type: 'api-error',
            message: `Failed to change viewer preference: ${result.errorReason} (${result.errorDescription})`,
          };
        }
      })
      .catch((error) => {
        console.error('Profile update error', error);
        errorModal = {
          type: 'api-error',
          message: `Failed to change viewer preference: ${
            error?.body?.message !== undefined ? error.body.message : error
          }`,
        };
      });
  }

  function updateNewsOptIn(optIn: boolean): void {
    apiUpdateProfile({newsOptIn: optIn})
      .then((result) => {
        if (result.success) {
          invalidateAll();
        } else {
          errorModal = {
            type: 'api-error',
            message: `Failed to change news status: ${result.errorReason} (${result.errorDescription})`,
          };
        }
      })
      .catch((error) => {
        console.error('Profile update error', error);
        errorModal = {
          type: 'api-error',
          message: `Failed to change news status: ${error?.body?.message !== undefined ? error.body.message : error}`,
        };
      });
  }

  /**
   * Change the user's default display units.
   */
  function updateUnits(altitudeUnit: string, distanceUnit: string): void {
    apiUpdateProfile({altitudeUnit, distanceUnit})
      .then((result) => {
        if (result.success) {
          // The saved preference wins over any session override now.
          clearSessionUnits();
          units.update((u) => ({
            ...u,
            altitudeUnit: altitudeUnit as AltitudeUnit,
            distanceUnit: distanceUnit as DistanceUnit,
          }));
          invalidateAll();
        } else {
          errorModal = {
            type: 'api-error',
            message: `Failed to change units: ${result.errorReason} (${result.errorDescription})`,
          };
        }
      })
      .catch((error) => {
        console.error('Profile update error', error);
        errorModal = {
          type: 'api-error',
          message: `Failed to change units: ${error?.body?.message !== undefined ? error.body.message : error}`,
        };
      });
  }
</script>

{#if errorModal?.type === 'api-error'}
  <MessageModal
    type="error"
    title={$i18n.t('common.error--api-error')}
    message={errorModal.message}
    showClose={true}
    on:closed={() => (errorModal = undefined)}
  />
{/if}

<nav class="breadcrumb" aria-label="breadcrumbs">
  <ul>
    <li><a href="/">{$i18n.t('navigation.home')}</a></li>
    <li class="is-active"><a href="./" aria-current="page">{$i18n.t('navigation.profile')}</a></li>
  </ul>
</nav>

<Flashes bind:this={flashes} />

<h2 class="title is-2">{$i18n.t('navigation.profile')}</h2>

<p class="block">
  <SubstitutableText text={$i18n.t('profile.prose--welcome')}>
    <strong slot="1">{data.profile.username}</strong>
  </SubstitutableText>
</p>

<p class="block">
  <SubstitutableText text={$i18n.t('profile.prose--change-password')}>
    <a slot="1" href="/auth/password/change/" let:text>{text}</a>
  </SubstitutableText>
</p>

<div class="block">
  <h3 class="title is-4">{$i18n.t('profile.title--user-info')}</h3>
  <table class="table is-hoverable">
    <tbody>
      <tr>
        <th>{$i18n.t('profile.title--username')}</th>
        <td>{data.profile.username}</td>
      </tr>
      <tr>
        <th>{$i18n.t('profile.title--email')}</th>
        <td>{data.profile.email}</td>
      </tr>
      <tr>
        <th>{$i18n.t('profile.title--registered-since')}</th>
        <td>{data.profile.signedUp}</td>
      </tr>
    </tbody>
  </table>
</div>

<div class="block">
  <h3 class="title is-4">{$i18n.t('profile.title--units')}</h3>

  <article class="message is-info">
    <div class="message-body">
      {$i18n.t('profile.prose--units')}
    </div>
  </article>

  <div class="field is-horizontal">
    <div class="field-label is-normal">
      <label class="label" for="altitude-unit">{$i18n.t('profile.title--altitude-unit')}</label>
    </div>
    <div class="field-body">
      <div class="select">
        <select
          id="altitude-unit"
          value={$units.altitudeUnit}
          on:change={(event) =>
            updateUnits(event.currentTarget.value as string, $units.distanceUnit)}
        >
          {#each ALTITUDE_UNITS as unit (unit)}
            <option value={unit}
              >{unit === 'm'
                ? $i18n.t('common.unit-name--m')
                : $i18n.t('common.unit-name--ft')}</option
            >
          {/each}
        </select>
      </div>
    </div>
  </div>

  <div class="field is-horizontal">
    <div class="field-label is-normal">
      <label class="label" for="distance-unit">{$i18n.t('profile.title--distance-unit')}</label>
    </div>
    <div class="field-body">
      <div class="select">
        <select
          id="distance-unit"
          value={$units.distanceUnit}
          on:change={(event) =>
            updateUnits($units.altitudeUnit, event.currentTarget.value as string)}
        >
          {#each DISTANCE_UNITS as unit (unit)}
            <option value={unit}
              >{unit === 'km'
                ? $i18n.t('common.unit-name--km')
                : unit === 'mi'
                  ? $i18n.t('common.unit-name--mi')
                  : $i18n.t('common.unit-name--nm')}</option
            >
          {/each}
        </select>
      </div>
    </div>
  </div>
</div>

{#if igcviewerEnabled}
  <div class="block">
    <h3 class="title is-4">{$i18n.t('profile.title--track-viewer')}</h3>

    <article class="message is-info">
      <div class="message-body">
        {$i18n.t('profile.prose--track-viewer')}
      </div>
    </article>

    <div class="field is-horizontal">
      <div class="field-label is-normal">
        <label class="label" for="viewer-new-tab">
          {$i18n.t('profile.title--viewer-new-tab')}
        </label>
      </div>
      <div class="field-body">
        <div class="control">
          <input
            id="viewer-new-tab"
            type="checkbox"
            checked={data.profile.openViewerInNewTab !== false}
            on:change={(event) => updateOpenViewerInNewTab(event.currentTarget.checked)}
          />
        </div>
      </div>
    </div>
  </div>
{/if}

<div class="block">
  <h3 class="title is-4">{$i18n.t('profile.title--news')}</h3>

  <article class="message is-info">
    <div class="message-body">
      {$i18n.t('profile.prose--news')}
    </div>
  </article>

  <div class="content">
    <p>
      {#if data.profile.newsOptIn}
        <SubstitutableText text={$i18n.t('profile.newsletter-allowed')}>
          <strong slot="1" let:text>{text}</strong>
        </SubstitutableText>
      {:else}
        <SubstitutableText text={$i18n.t('profile.newsletter-disallowed')}>
          <strong slot="1" let:text>{text}</strong>
        </SubstitutableText>{/if}
    </p>
  </div>

  <div class="block">
    <button
      class="button"
      class:is-warning={data.profile.newsOptIn}
      class:is-primary={!data.profile.newsOptIn}
      on:click={() => updateNewsOptIn(!data.profile.newsOptIn)}
    >
      {data.profile.newsOptIn
        ? $i18n.t('profile.newsletter-unsubscribe')
        : $i18n.t('profile.newsletter-subscribe')}
    </button>
  </div>
</div>
