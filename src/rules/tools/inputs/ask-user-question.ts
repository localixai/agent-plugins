/**
 * AskUserQuestion's input (hooks.md, "AskUserQuestion"): "one to four" questions, "each with a
 * `question` string, short `header`, `options` array, and optional `multiSelect` flag".
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/tools/inputs/ask_user_question.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../../../format/issues.ts';
import type { AskUserQuestionInput } from '../../../protocol/official-tools.ts';
import { coreModule } from '../../../wasm/bridge.ts';

const call = coreModule('rules/tools/inputs/ask-user-question');

type OfficialQuestion = AskUserQuestionInput['questions'][number];

/**
 * What `answerQuestions` needs of an AskUserQuestion input, taken from the SDK's
 * `AskUserQuestionInput`. `multiSelect` is optional here, as hooks.md says ("optional `multiSelect`
 * flag"), where the SDK requires it (spec/judgments.md).
 */
export interface AskedQuestions extends Pick<AskUserQuestionInput, 'answers'> {
  readonly questions: readonly (Pick<OfficialQuestion, 'question'> &
    Partial<Pick<OfficialQuestion, 'multiSelect'>>)[];
}

export interface AnsweredQuestions<T extends AskedQuestions> {
  /** The whole input with `answers`, ready to be a hook's `updatedInput`. */
  readonly input: T & { readonly answers: Readonly<Record<string, string>> };
  readonly issues: readonly PluginIssue[];
}

/**
 * Answers a call programmatically (hooks.md, "Tools that require user interaction"): "echo back the
 * original `questions` array and add an `answers` object mapping each question's text to the chosen
 * answer", the labels of a multi-select question joined with commas. `selections` maps a question's
 * text to what the user chose. Judgments (spec/judgments.md): a selection for a question that was
 * not asked, or two or more for a single-select question, is an issue and adds nothing; no
 * selection leaves the question unanswered; a text that is no option label stays, because the user
 * can write their own answer; answers already in the input stay unless a selection replaces them.
 */
export const answerQuestions = <T extends AskedQuestions>(
  input: T,
  selections: Readonly<Record<string, readonly string[]>>,
): AnsweredQuestions<T> => call('answerQuestions', input, selections);
