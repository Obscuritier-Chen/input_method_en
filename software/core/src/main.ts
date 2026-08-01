import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface CandidateDto {
  word: string;
  score: number;
}

const editor = document.querySelector<HTMLInputElement>("#editor")!;
const candidatesEl = document.querySelector<HTMLDivElement>("#candidates")!;

function splitContextAndPrefix(text: string): { context: string[]; prefix: string } {

  const trimmed = text.trimEnd();

  if (trimmed.length === 0) {
    return { context: [], prefix: "" };
  }

  const endsWithSpace = text.endsWith(" ");

  const words = trimmed.toLowerCase().split(/\s+/).filter(Boolean);

  if (endsWithSpace) {
    // 用户刚打完一个词又打了空格，说明当前没有正在输入的前缀
    return { context: words, prefix: "" };
  }

  // 最后一个词是正在输入、还没完成的部分，其余是已确定的上下文
  const prefix = words[words.length - 1] ?? "";
  const context = words.slice(0, -1);

  return { context, prefix };
}

async function updateCandidates() {

  const { context, prefix } = splitContextAndPrefix(editor.value);

  if (prefix.length === 0) {
    await invoke("hide_candidates_window");
    return;
  }

  try {

    const results = await invoke<CandidateDto[]>("get_candidates", { context, prefix });

    if (results.length === 0) {
      await invoke("hide_candidates_window");
      return;
    }

    // 把输入框在屏幕上的物理坐标算出来，告诉候选窗口该出现在哪
    const mainWin = getCurrentWindow();
    const winPos = await mainWin.outerPosition();
    const scaleFactor = await mainWin.scaleFactor();
    const rect = editor.getBoundingClientRect();

    const x = winPos.x + rect.left * scaleFactor;
    const y = winPos.y + rect.bottom * scaleFactor + 4;

    await invoke("show_candidates_window", { x, y, candidates: results });

  } catch (err) {

    console.error(err);

  }
}

function renderCandidates(results: CandidateDto[]) {

  candidatesEl.innerHTML = "";

  if (results.length === 0) {
    candidatesEl.innerHTML = `<span style="color:#888">无候选</span>`;
    return;
  }

  for (const candidate of results) {

    const el = document.createElement("div");
    el.className = "candidate";
    el.innerHTML = `${candidate.word}<span class="score">${candidate.score.toFixed(2)}</span>`;

    candidatesEl.appendChild(el);
  }
}

editor.addEventListener("input", () => {

  updateCandidates();

});