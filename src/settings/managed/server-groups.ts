import type { Settings } from '../../protocol/official.ts';

/**
 * Group configurations of server-managed settings (spec §14.3.7; an Enterprise plan). Which
 * configuration a member receives is decided where the settings are delivered; this is that
 * decision over plain data, for a host or gateway that delivers them. To the client the delivered
 * configuration is the whole server-managed source (§14.2).
 */
export interface GroupConfiguration {
  /** A group the identity provider syncs through SCIM, or a custom group of the admin console. */
  readonly group: string;
  readonly settings: Settings;
}

export interface GroupedDelivery {
  /** The organization switched to group-based mode (never with HIPAA enabled). */
  readonly groupBased: boolean;
  /** The organization-wide configuration. */
  readonly organization?: Settings | undefined;
  /** The groups' configurations, highest rank first. */
  readonly groups: readonly GroupConfiguration[];
  /** The configuration of everyone no ranked group covers. */
  readonly catchAll?: Settings | undefined;
}

/** The session the configuration is for: a local session, or a cloud or remote Cowork one. */
export interface DeliverySession {
  readonly local: boolean;
}

function forMember(delivery: GroupedDelivery, groups: readonly string[]): Settings | undefined {
  if (!delivery.groupBased) return delivery.organization;
  const ranked = delivery.groups.find((configuration) => groups.includes(configuration.group));
  return ranked?.settings ?? delivery.catchAll;
}

/**
 * "A group's `availableModels` holds only in local sessions; cloud and remote Cowork sessions take
 * `availableModels` from the catch-all, or from the organization-wide configuration when no
 * catch-all is set, even in group-based mode."
 */
function withCloudModels(settings: Settings, delivery: GroupedDelivery): Settings {
  const source = delivery.catchAll ?? delivery.organization;
  const rest: Settings = Object.fromEntries(
    Object.entries(settings).filter(([name]) => name !== 'availableModels'),
  );
  const models = source?.availableModels;
  return models === undefined ? rest : { ...rest, availableModels: models };
}

/**
 * The configuration a member receives: the highest-ranked one among their groups, else the
 * catch-all, else none (only endpoint-managed settings apply); never a merge of two.
 */
export function deliveredConfiguration(
  delivery: GroupedDelivery,
  member: { readonly groups: readonly string[] },
  session: DeliverySession,
): Settings | undefined {
  const settings = forMember(delivery, member.groups);
  if (settings === undefined || session.local || !delivery.groupBased) return settings;
  return withCloudModels(settings, delivery);
}
