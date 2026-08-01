import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";

interface CandidateDto {
  word: string;
  score: number;
}

const panel = document.querySelector<HTMLDivElement>("#panel")!;

let currentCandidates: CandidateDto[] = [];
let selectedIndex = 0;

function render() {

  panel.innerHTML = "";

  currentCandidates.forEach((c, i) => {

    const el = document.createElement("div");
    el.className = "item" + (i === selectedIndex ? " selected" : "");
    el.innerHTML = `<span class="idx">${i + 1}</span>${c.word}`;

    el.addEventListener("click", () => selectCandidate(i));

    panel.appendChild(el);
  });

  // 内容变化后自适应窗口大小，避免固定尺寸导致裁切或留白
  requestAnimationFrame(() => {

    const rect = panel.getBoundingClientRect();

    getCurrentWindow().setSize(
      new (window as any).__TAURI__.dpi.LogicalSize(
        Math.max(rect.width, 10),
        Math.max(rect.height, 10)
      )
    );

  });
}

async function selectCandidate(index: number) {

  const candidate = currentCandidates[index];

  if (!candidate) return;

  // 现阶段先只把选中结果发回主窗口打印验证，
  // 真正“把词提交进正在输入的应用”这一步要等 TSF 接入后才有意义
  await invoke("on_candidate_selected", { word: candidate.word });

  await getCurrentWindow().hide();
}

getCurrentWindow().listen<CandidateDto[]>("candidates-updated", (event) => {

  currentCandidates = event.payload;
  selectedIndex = 0;

  render();

});

// 键盘上下选择、回车确认，为将来接 TSF 的按键转发预留同样的交互逻辑
window.addEventListener("keydown", (e) => {

  if (e.key === "ArrowDown") {
    selectedIndex = Math.min(selectedIndex + 1, currentCandidates.length - 1);
    render();
  } else if (e.key === "ArrowUp") {
    selectedIndex = Math.max(selectedIndex - 1, 0);
    render();
  } else if (e.key === "Enter") {
    selectCandidate(selectedIndex);
  } else if (e.key === "Escape") {
    getCurrentWindow().hide();
  }

});