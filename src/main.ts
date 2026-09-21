import { invoke } from "@tauri-apps/api/core";

interface Draft {
  draft: string;
  verify: string[];
  model: string;
  created_at: string;
}

interface Question {
  id: string;
  asked_at: string;
  asker: string;
  context: string;
  question: string;
  tags: string[];
  draft: Draft | null;
}

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

const form = $<HTMLFormElement>("q-form");
const fQuestion = $<HTMLTextAreaElement>("f-question");
const fAsker = $<HTMLInputElement>("f-asker");
const fContext = $<HTMLInputElement>("f-context");
const fTags = $<HTMLInputElement>("f-tags");
const saveBtn = $<HTMLButtonElement>("save-btn");
const errorBox = $<HTMLParagraphElement>("error");
const emptyMsg = $<HTMLParagraphElement>("empty");
const list = $<HTMLUListElement>("list");

let questions: Question[] = [];
// Per-row UI state that must survive re-renders.
const busy = new Set<string>(); // ids currently drafting
const confirming = new Set<string>(); // ids awaiting delete confirmation
const rowErrors = new Map<string, string>();

function errText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : "Something went wrong.";
}

function showError(msg: string) {
  errorBox.textContent = msg;
  errorBox.hidden = false;
}
function clearError() {
  errorBox.hidden = true;
  errorBox.textContent = "";
}

function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  opts: { cls?: string; text?: string } = {},
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  if (opts.cls) node.className = opts.cls;
  if (opts.text !== undefined) node.textContent = opts.text; // never innerHTML: user text stays text
  return node;
}

function fmtDate(iso: string): string {
  const d = new Date(iso);
  return isNaN(d.getTime()) ? iso : d.toLocaleString();
}

function renderDraft(d: Draft): HTMLElement {
  const box = el("div", { cls: "draft" });
  box.appendChild(el("p", { cls: "draft-label", text: "AI draft — verify before use" }));
  box.appendChild(el("p", { text: d.draft }));
  box.appendChild(el("h3", { text: "Verify these" }));
  const ul = el("ul");
  for (const v of d.verify) ul.appendChild(el("li", { text: v }));
  box.appendChild(ul);
  return box;
}

function renderRow(q: Question): HTMLElement {
  const li = el("li", { cls: "q" });
  li.appendChild(el("p", { cls: "q-text", text: q.question }));
  const who = [q.asker, q.context].filter(Boolean).join(" · ");
  li.appendChild(el("p", { cls: "meta", text: [who, fmtDate(q.asked_at)].filter(Boolean).join(" — ") }));
  if (q.tags.length) {
    const tags = el("p", { cls: "tags" });
    for (const t of q.tags) tags.appendChild(el("span", { cls: "tag", text: t }));
    li.appendChild(tags);
  }

  const actions = el("div", { cls: "actions" });

  const draftBtn = el("button", { text: busy.has(q.id) ? "Drafting…" : q.draft ? "Redraft" : "Draft" });
  draftBtn.disabled = busy.has(q.id);
  draftBtn.addEventListener("click", () => onDraft(q.id));
  actions.appendChild(draftBtn);

  if (confirming.has(q.id)) {
    const yes = el("button", { cls: "danger", text: "Confirm delete" });
    yes.addEventListener("click", () => onDelete(q.id));
    const no = el("button", { text: "Cancel" });
    no.addEventListener("click", () => {
      confirming.delete(q.id);
      render();
    });
    actions.append(yes, no);
  } else {
    const del = el("button", { cls: "danger", text: "Delete" });
    del.addEventListener("click", () => {
      confirming.add(q.id);
      render();
    });
    actions.appendChild(del);
  }
  li.appendChild(actions);

  const err = rowErrors.get(q.id);
  if (err) li.appendChild(el("p", { cls: "row-error", text: err }));
  if (q.draft) li.appendChild(renderDraft(q.draft));
  return li;
}

function render() {
  list.replaceChildren(...questions.map(renderRow));
  emptyMsg.hidden = questions.length > 0;
}

async function refresh() {
  try {
    questions = await invoke<Question[]>("list_questions");
    render();
  } catch (e) {
    showError(`Could not load your questions: ${errText(e)}`);
  }
}

async function onDraft(id: string) {
  busy.add(id);
  rowErrors.delete(id);
  render();
  try {
    const updated = await invoke<Question>("draft_answer", { id });
    questions = questions.map((q) => (q.id === id ? updated : q));
  } catch (e) {
    rowErrors.set(id, errText(e));
  } finally {
    busy.delete(id);
    render();
  }
}

async function onDelete(id: string) {
  confirming.delete(id);
  try {
    await invoke("delete_question", { id });
    questions = questions.filter((q) => q.id !== id);
    rowErrors.delete(id);
    clearError();
  } catch (e) {
    showError(`Could not delete the question: ${errText(e)}`);
  }
  render();
}

form.addEventListener("submit", async (ev) => {
  ev.preventDefault();
  clearError();
  const question = fQuestion.value.trim();
  if (!question) {
    showError("Please type the question before saving.");
    return;
  }
  const tags = fTags.value
    .split(",")
    .map((t) => t.trim())
    .filter(Boolean);
  saveBtn.disabled = true;
  try {
    const saved = await invoke<Question>("save_question", {
      asker: fAsker.value.trim(),
      context: fContext.value.trim(),
      question,
      tags,
    });
    questions = [saved, ...questions];
    form.reset();
    render();
    fQuestion.focus();
  } catch (e) {
    showError(`Could not save the question: ${errText(e)}`);
  } finally {
    saveBtn.disabled = false;
  }
});

refresh();
