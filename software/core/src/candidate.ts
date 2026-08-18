import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

interface CandidateDto {
    word: string;
    score: number;
}

interface CursorRect {
    left: number;
    top: number;
    right: number;
    bottom: number;
}

interface UpdateContext {
    session_id: number;
    prefix: string;
    buffer: string;
    cursor_rect?: CursorRect | null;
}

interface ClientRequest {
    UpdateContext?: UpdateContext;
    CancelComposition?: {
        session_id: number;
    };
}

interface CandidatesResponse {
    session_id: number;
    prefix: string;
    candidates: CandidateDto[];
}

let currentSessionId: number | null = null;
let currentPrefix = "";
let currentCandidates: CandidateDto[] = [];

let unlistenContext: UnlistenFn | null = null;

const prefixValue = getRequiredElement<HTMLSpanElement>(
    "prefix-value",
);

const candidatesContainer =
    getRequiredElement<HTMLElement>(
        "candidates",
    );

function getRequiredElement<T extends HTMLElement>(
    id: string,
): T {
    const element = document.getElementById(id);

    if (!element) {
        throw new Error(
            `Element #${id} not found`,
        );
    }

    return element as T;
}

function escapeHtml(text: string): string {
    const div = document.createElement("div");
    div.textContent = text;
    return div.innerHTML;
}

function updatePrefix(prefix: string): void {
    currentPrefix = prefix;
    prefixValue.textContent = prefix;
}

function clearCandidates(): void {
    currentCandidates = [];
    candidatesContainer.innerHTML = "";
}

function renderCandidates(
    candidates: CandidateDto[],
): void {
    currentCandidates = candidates.slice(0, 5);

    candidatesContainer.innerHTML = "";

    currentCandidates.forEach(
        (candidate, index) => {
            const item =
                document.createElement("button");

            item.type = "button";
            item.className = "candidate";

            item.dataset.index =
                String(index);

            item.innerHTML = `
                <span class="candidate-number">
                    ${index + 1}
                </span>

                <span class="candidate-word">
                    ${escapeHtml(candidate.word)}
                </span>

                <span class="candidate-score">
                    ${candidate.score.toFixed(3)}
                </span>
            `;

            item.addEventListener(
                "click",
                () => {
                    void selectCandidate(index);
                },
            );

            candidatesContainer.appendChild(
                item,
            );
        },
    );
}

async function requestCandidates(
    buffer: string,
    prefix: string,
): Promise<void> {
    try {
        const candidates =
            await invoke<CandidateDto[]>(
                "get_candidates",
                {
                    buffer,
                    prefix,
                },
            );

        renderCandidates(candidates);
    } catch (error) {
        console.error(
            "[Candidate] get_candidates failed:",
            error,
        );

        clearCandidates();
    }
}

function renderNoCandidate(): void {
    const container = document.getElementById("candidates");

    if (!container) {
        return;
    }

    container.innerHTML = `
        <div class="no-candidate">
            No candidate
        </div>
    `;
}

async function handleUpdateContext(
    payload: ClientRequest,
): Promise<void> {
    const update =
        payload.UpdateContext;

    if (!update) {
        return;
    }

    currentSessionId =
        update.session_id;

    console.log(`prefix: ${update.prefix}, buffer: ${update.buffer}`);

    const prefix =
        update.prefix ?? "";

    const buffer =
        update.buffer ?? "";

    updatePrefix(buffer);//依旧gemini/chatgpt 与 claude 接口不一致导致 buffer<->prefix context<->prefix

    try {
        const candidates =
            await invoke<CandidateDto[]>(
                "get_candidates",
                {
                    buffer,
                    prefix,
                },
            );

        currentCandidates =
            candidates.slice(0, 5);

        renderCandidates(currentCandidates);

        if (currentCandidates.length === 0) {
            renderNoCandidate();

            const rect = update.cursor_rect;

            if (rect) {
                await invoke("show_candidates_window", {
                    x: rect.left,
                    y: rect.bottom,
                });
            }

            return;
        }

        // 显示候选窗口
        if (update.cursor_rect) {
            await invoke(
                "show_candidates_window",
                {
                    x: update.cursor_rect.left,
                    y: update.cursor_rect.bottom,
                },
            );
        } else {
            // 没有光标位置，仍然显示
            await invoke(
                "show_candidates_window",
                {
                    x: 0,
                    y: 0,
                    candidates:
                        currentCandidates,
                },
            );
        }
    } catch (error) {
        console.error(
            "[Candidate] failed to update:",
            error,
        );

        clearCandidates();
    }
}

async function hideCandidateWindow(): Promise<void> {
    currentSessionId = null;
    currentPrefix = "";
    currentCandidates = [];

    prefixValue.textContent = "";
    clearCandidates();

    try {
        await invoke(
            "hide_candidates_window",
        );
    } catch (error) {
        console.error(
            "[Candidate] hide window failed:",
            error,
        );
    }
}

async function handleCancelComposition(
    payload: ClientRequest,
): Promise<void> {
    const cancel =
        payload.CancelComposition;

    if (!cancel) {
        return;
    }

    if (
        currentSessionId !== null &&
        cancel.session_id !== currentSessionId
    ) {
        return;
    }

    await hideCandidateWindow();
}

async function selectCandidate(
    index: number,
): Promise<void> {
    if (
        currentSessionId === null
    ) {
        console.warn(
            "[Candidate] no active session",
        );
        return;
    }

    const candidate =
        currentCandidates[index];

    if (!candidate) {
        return;
    }

    try {
        await invoke(
            "on_candidate_selected",
            {
                sessionId:
                    currentSessionId,

                word:
                    candidate.word,
            },
        );

        clearCandidates();
    } catch (error) {
        console.error(
            "[Candidate] selection failed:",
            error,
        );
    }
}

function handleKeyDown(
    event: KeyboardEvent,
): void {
    if (
        event.key < "1" ||
        event.key > "5"
    ) {
        return;
    }

    const index =
        Number(event.key) - 1;

    if (!currentCandidates[index]) {
        return;
    }

    event.preventDefault();

    void selectCandidate(index);
}

let unlistenCancelComposition: UnlistenFn | null = null;
async function init(): Promise<void> {
    console.log("[Candidate] init()");

    try {
        unlistenContext = await listen<ClientRequest>(
            "ime-update-context",
            (event) => {

                void handleUpdateContext(
                    event.payload,
                );
            },
        );

        unlistenCancelComposition = await listen<ClientRequest>(
            "ime-cancel-composition",
            (event) => {
                void handleCancelComposition(
                    event.payload,
                );
            },
        );

    } catch (error) {
        console.error(
            "[Candidate] listener registration failed:",
            error,
        );
    }

    window.addEventListener(
        "keydown",
        handleKeyDown,
    );
}

void init();