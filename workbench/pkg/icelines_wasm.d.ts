/* tslint:disable */
/* eslint-disable */

export class BrowserEngine {
    free(): void;
    [Symbol.dispose](): void;
    current_season(): number;
    export_package(): Uint8Array;
    /**
     * Stage and validate before replacing the active repository.
     */
    load_package(bytes: Uint8Array): string;
    constructor();
    player_detail(player_id: number): string;
    query(request: string): string;
    resident_revisions(): string;
    /**
     * Stateless projection: schedule acquisition never replaces season stats.
     */
    schedule_week(bytes: Uint8Array): string;
    /**
     * Select an already validated window by exact revision; never fetch or save.
     */
    select_resident(revision: string): void;
    unload_active(): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_browserengine_free: (a: number, b: number) => void;
    readonly browserengine_current_season: (a: number) => number;
    readonly browserengine_export_package: (a: number) => [number, number, number, number];
    readonly browserengine_load_package: (a: number, b: number, c: number) => [number, number, number, number];
    readonly browserengine_new: () => number;
    readonly browserengine_player_detail: (a: number, b: number) => [number, number, number, number];
    readonly browserengine_query: (a: number, b: number, c: number) => [number, number, number, number];
    readonly browserengine_resident_revisions: (a: number) => [number, number];
    readonly browserengine_schedule_week: (a: number, b: number, c: number) => [number, number, number, number];
    readonly browserengine_select_resident: (a: number, b: number, c: number) => [number, number];
    readonly browserengine_unload_active: (a: number) => void;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
