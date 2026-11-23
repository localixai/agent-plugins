import { firstProblem, textOf, type ElementDef } from './def.ts';

/** `label` "Not blank" (`ProgressProps`, claude-code/index.d.ts:10940 of 2.1.330). */
const labelProblem = (label: unknown): string | undefined =>
  typeof label === 'string' && label.trim() !== ''
    ? undefined
    : 'Progress prop "label" must not be blank';

/**
 * `Progress`: how far along a task is, a leaf. In the 2.1.330 declarations' tables for every
 * surface and not in the documentation's (Agent Plugins Extended §10.10, Judgment J10.24). A `value`
 * absent or not finite draws as running, and past either end as that end, so it is never refused.
 */
export const PROGRESS: ElementDef = {
  name: 'Progress',
  props: ['label', 'value', 'max', 'valueText', 'color'],
  children: 'none',
  check: (props) => firstProblem(labelProblem(props.label), textOf('Progress', props, 'valueText')),
};
