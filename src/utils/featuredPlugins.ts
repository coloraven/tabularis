import type { PluginManifest } from '../types/plugins';
import type { CatalogueDriver } from './connectionCatalogue';

/**
 * Drivers that should appear in New Connection even before they exist on the
 * public registry (first-class catalogue presence). Once the plugin is
 * installed or listed by the registry, those sources win and this seed is skipped.
 */
export function featuredCatalogueDrivers(
  registered: PluginManifest[],
  registrySlugs: Set<string>,
): CatalogueDriver[] {
  const registeredIds = new Set(registered.map((d) => d.id));
  const out: CatalogueDriver[] = [];

  if (!registeredIds.has('spreadsheet') && !registrySlugs.has('spreadsheet')) {
    out.push({
      slug: 'spreadsheet',
      name: 'Excel / Spreadsheet',
      engine: 'spreadsheet',
      paradigms: ['sql'],
      verified: false,
      installed: false,
      installedVersion: null,
      latestVersion: '0.1.0',
      isBuiltin: false,
      platformSupported: true,
      downloads: null,
      updateAvailable: false,
      icon: null,
      color: '#217346',
    });
  }

  return out;
}
