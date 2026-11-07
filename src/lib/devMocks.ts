import { mockIPC } from "@tauri-apps/api/mocks";
import type { ToolInfo } from "@/features/home/tools";

/**
 * Stand-in backend for `bun run dev` in a plain browser, so screens can be
 * checked without the Tauri shell. Never part of a build. The Tools mirror
 * crates/rantakit-tasks/src/catalogue.rs on desktop.
 *
 * URL switches: `?recent` fills Recent tools, `?fail` makes the second file
 * of every Task miss its Size limit, `?slow` makes Tasks take longer,
 * `?historyOff` starts with History turned off, `?update` finds version 0.3.0
 * (with `?updateDeb`, as a Linux .deb that can't install it), `?pages` makes
 * each PDF a folder of 12 page files, as Pisah PDF will (X3).
 */
const TOOLS: ToolInfo[] = [
  { id: "kompres-foto", job: "kecilkan-ubah", comingSoon: false },
  { id: "kompres-pdf", job: "kecilkan-ubah", comingSoon: false },
  { id: "ubah-format-foto", job: "kecilkan-ubah", comingSoon: false },
  { id: "ubah-ukuran-foto", job: "kecilkan-ubah", comingSoon: false },
  { id: "gabung-pdf", job: "atur-pdf", comingSoon: false },
  { id: "pisah-pdf", job: "atur-pdf", comingSoon: false },
  { id: "atur-halaman-pdf", job: "atur-pdf", comingSoon: false },
  ...(
    [
      ["kompres-dokumen", "kecilkan-ubah"],
      ["kompres-audio", "kecilkan-ubah"],
      ["ubah-format-audio", "kecilkan-ubah"],
      ["kompres-video", "kecilkan-ubah"],
      ["word-ke-pdf", "kecilkan-ubah"],
      ["excel-ke-pdf", "kecilkan-ubah"],
      ["jpg-ke-pdf", "scan"],
      ["pdf-ke-jpg", "atur-pdf"],
      ["beri-watermark", "bagikan-aman"],
      ["hapus-info-lokasi", "bagikan-aman"],
    ] as const
  ).map(([id, job]) => ({ id, job, comingSoon: true })),
];

const FOLDER = "C:\\Users\\Rani\\Documents\\Lamaran CPNS\\";

/** What the file pickers return in the browser. */
export function devPick(noun: "photo" | "pdf"): Promise<string[]> {
  const names = noun === "photo" ? ["KTP.jpg", "Pas foto.png", "Ijazah scan.jpg"] : ["Ijazah.pdf", "Transkrip nilai.pdf"];
  return Promise.resolve(names.map((name) => FOLDER + name));
}

const has = (flag: string) => new URLSearchParams(location.search).has(flag);

/** A made-up but steady size for a path. */
function sizeOf(path: string) {
  let hash = 0;
  for (const char of path) hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
  return 600_000 + (hash % 3_000_000);
}

const isPdf = (path: string) => path.toLowerCase().endsWith(".pdf");
const longestSideOf = (path: string) => 600 + (sizeOf(path) % 3400);
const SIDES: Record<string, number> = { large: 1920, medium: 1280, small: 800 };
const kindOf = (path: string) => (isPdf(path) ? "pdf" : path.toLowerCase().endsWith(".png") ? "png" : "jpg");

/** A soft gradient as a JPG, in place of a real thumbnail. */
async function fakeThumbnail(path: string, size: number) {
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = Math.round(size * 0.75);
  const context = canvas.getContext("2d")!;
  const hue = sizeOf(path) % 360;
  const gradient = context.createLinearGradient(0, 0, canvas.width, canvas.height);
  gradient.addColorStop(0, `hsl(${hue} 60% 75%)`);
  gradient.addColorStop(1, `hsl(${(hue + 60) % 360} 50% 45%)`);
  context.fillStyle = gradient;
  context.fillRect(0, 0, canvas.width, canvas.height);
  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/jpeg"));
  return blob ? await blob.arrayBuffer() : new ArrayBuffer(0);
}

/** A white A4 page with a heading and grey lines, drawn after a moment like a real page. */
async function fakePage(path: string, page: number, size: number) {
  await new Promise((resolve) => setTimeout(resolve, 150 + (page % 4) * 80));
  const canvas = document.createElement("canvas");
  canvas.height = size;
  canvas.width = Math.round(size / 1.414);
  const context = canvas.getContext("2d")!;
  const unit = canvas.width / 20;
  context.fillStyle = "#ffffff";
  context.fillRect(0, 0, canvas.width, canvas.height);
  context.fillStyle = `hsl(${(sizeOf(path) + page * 40) % 360} 45% 55%)`;
  context.fillRect(2 * unit, 2 * unit, 9 * unit, 1.4 * unit);
  context.fillStyle = "#cbd5e1";
  for (let line = 0; line < 14; line++) {
    const width = 16 - ((line * 7 + page) % 5);
    context.fillRect(2 * unit, (5.5 + line * 1.6) * unit, width * unit, 0.6 * unit);
  }
  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/jpeg"));
  return blob ? await blob.arrayBuffer() : new ArrayBuffer(0);
}

/** A day of Tasks, one from yesterday, and an older one whose Result was moved. */
function sampleHistory() {
  const now = Date.now();
  const day = 24 * 60 * 60 * 1000;
  const file = (name: string, suffix: string, exists = true) => {
    const dot = name.lastIndexOf(".");
    const originalPath = FOLDER + name;
    return {
      originalPath,
      resultPath: `${FOLDER}${name.slice(0, dot)} (${suffix})${name.slice(dot)}`,
      originalBytes: sizeOf(originalPath),
      resultBytes: Math.round(sizeOf(originalPath) * 0.15),
      exists,
    };
  };
  return [
    {
      id: "3",
      tool: "kompres-foto",
      createdAt: now - 60 * 60 * 1000,
      files: ["KTP.jpg", "Pas foto.jpg", "Ijazah scan.jpg"].map((name) => file(name, "compressed")),
    },
    {
      id: "4",
      tool: "pisah-pdf",
      createdAt: now - 2 * 60 * 60 * 1000,
      files: [1, 2, 3].map((page) => ({
        originalPath: `${FOLDER}Ijazah.pdf`,
        resultPath: `${FOLDER}Ijazah (pages)\\Ijazah (page ${page}).pdf`,
        originalBytes: sizeOf(`${FOLDER}Ijazah.pdf`),
        resultBytes: Math.round(sizeOf(`${FOLDER}Ijazah.pdf`) / 3),
        exists: true,
      })),
    },
    {
      id: "5",
      tool: "gabung-pdf",
      createdAt: now - 3 * 60 * 60 * 1000,
      files: ["Ijazah.pdf", "Transkrip nilai.pdf"].map((name) => ({
        originalPath: FOLDER + name,
        resultPath: `${FOLDER}merged 2026-11-07.pdf`,
        originalBytes: sizeOf(FOLDER + name),
        resultBytes: 3_100_000,
        exists: true,
      })),
    },
    { id: "2", tool: "kompres-pdf", createdAt: now - day, files: [file("Ijazah.pdf", "compressed")] },
    { id: "1", tool: "ubah-format-foto", createdAt: now - 9 * day, files: [file("Logo.jpg", "converted", false)] },
  ];
}

const appSettings = { keepHistory: !has("historyOff"), resultsFolder: null as string | null, checkUpdates: true };
const cancelled = new Set<number>();
const recentLimits = [200 * 1024, 1024 * 1024, 500 * 1024];
const presets: { id: string; tool: string; name: string; options: object }[] = [];
let nextTask = 1;

interface TaskArgs {
  request: {
    options: {
      tool: string;
      to?: string;
      size?: string;
      maxSide?: number;
      limit?: number;
      date?: string;
      pages?: number[] | { page: number; turns: number }[];
      every?: number;
    };
    paths: string[];
  };
  onEvent: { onmessage: (message: unknown) => void };
}

/** How many pages a made-up PDF has: 2 to 12. */
const pageCountOf = (path: string) => 2 + (sizeOf(path) % 11);

/** "pages 1-3, 5", as `pages_suffix` in crates/rantakit-tasks/src/names.rs. */
function pagesSuffix(pages: number[]) {
  const runs: string[] = [];
  for (let i = 0; i < pages.length; ) {
    let j = i;
    while (j + 1 < pages.length && pages[j + 1] === pages[j] + 1) j++;
    runs.push(j > i ? `${pages[i]}-${pages[j]}` : `${pages[i]}`);
    i = j + 1;
  }
  return `${pages.length === 1 ? "page" : "pages"} ${runs.join(", ")}`;
}

/** Gabung PDF, Pisah PDF and Atur Halaman PDF, which work on pages. */
async function runPageTask(id: number, { options, paths }: TaskArgs["request"], send: (message: unknown) => void) {
  const step = has("slow") ? 400 : 120;
  const stem = (path: string) => path.slice(0, path.lastIndexOf("."));
  const progress = async (file: number) => {
    for (let fraction = 0; fraction < 1; fraction += 0.2) {
      if (cancelled.has(id)) return false;
      send({ event: "progress", file, fraction });
      await new Promise((resolve) => setTimeout(resolve, step));
    }
    return !cancelled.has(id);
  };
  const done = (file: number, result: object) =>
    send({ event: "fileDone", file, outcome: { status: "done", result: { inputPath: paths[file], ...result } } });
  const failed = (file: number, code: string) =>
    send({ event: "fileDone", file, outcome: { status: "failed", error: { code } } });

  if (options.tool === "gabung-pdf") {
    for (const file of paths.keys()) {
      if (!(await progress(file))) {
        paths.forEach((_, i) => failed(i, "cancelled"));
        return;
      }
      // `?fail`: the second PDF is locked, so nothing is made.
      if (has("fail") && file === 1) {
        paths.forEach((_, i) => failed(i, i === 1 ? "password-protected" : "cancelled"));
        return;
      }
    }
    const pages = paths.reduce((sum, path) => sum + pageCountOf(path), 0);
    const outputPath = `${paths[0].slice(0, paths[0].lastIndexOf("\\") + 1)}merged ${options.date}.pdf`;
    const bytes = paths.reduce((sum, path) => sum + sizeOf(path), 0);
    paths.forEach((path, file) =>
      done(file, { outputPath, originalBytes: sizeOf(path), compressedBytes: Math.round(bytes * 0.9), pages }),
    );
    return;
  }

  for (const [file, path] of paths.entries()) {
    if (!(await progress(file))) {
      failed(file, "cancelled");
      continue;
    }
    const count = pageCountOf(path);
    const size = sizeOf(path);
    if (options.tool === "atur-halaman-pdf") {
      const pages = (options.pages ?? []) as { page: number; turns: number }[];
      done(file, { outputPath: `${stem(path)} (edited).pdf`, originalBytes: size, compressedBytes: size, pages: pages.length });
    } else if (options.pages) {
      const asked = options.pages as number[];
      const kept = asked.filter((page) => page <= count);
      const skippedPages = asked.filter((page) => page > count);
      if (kept.length === 0) failed(file, "no-such-pages");
      else
        done(file, {
          outputPath: `${stem(path)} (${pagesSuffix(kept)}).pdf`,
          originalBytes: size,
          compressedBytes: Math.round((size * kept.length) / count),
          pages: kept.length,
          skippedPages,
        });
    } else {
      const every = options.every ?? 1;
      if (every >= count) {
        failed(file, "unchanged");
        continue;
      }
      const folder = `${stem(path)} (pages)`;
      const name = folder.slice(folder.lastIndexOf("\\") + 1, -" (pages)".length);
      const files = Array.from({ length: Math.ceil(count / every) }, (_, part) => {
        const pages = Array.from({ length: Math.min(every, count - part * every) }, (_, i) => part * every + i + 1);
        return { path: `${folder}\\${name} (${pagesSuffix(pages)}).pdf`, bytes: Math.round((size * pages.length) / count) };
      });
      done(file, {
        outputPath: folder,
        originalBytes: size,
        compressedBytes: files.reduce((sum, part) => sum + part.bytes, 0),
        files,
      });
    }
  }
}

/** One PDF split into 12 pages, in a new folder named after it. */
function pagesOf(path: string, size: number) {
  const dot = path.lastIndexOf(".");
  const name = path.slice(FOLDER.length, dot);
  const folder = `${FOLDER}${name} (pages)`;
  const files = Array.from({ length: 12 }, (_, page) => ({
    path: `${folder}\\${name} (page ${page + 1}).pdf`,
    bytes: Math.round(size / 12),
  }));
  return { outputPath: folder, compressedBytes: files.reduce((sum, file) => sum + file.bytes, 0), files };
}

function runTask({ request, onEvent }: TaskArgs) {
  const id = nextTask++;
  const { limit } = request.options;
  if (limit) recentLimits.splice(0, Infinity, ...[limit, ...recentLimits.filter((old) => old !== limit)].slice(0, 5));
  const send = (message: unknown) => onEvent.onmessage(message);
  const step = has("slow") ? 400 : 120;
  const suffix = request.options.tool.startsWith("ubah-format")
    ? "converted"
    : request.options.tool.startsWith("ubah-ukuran")
      ? "resized"
      : "compressed";

  if (["gabung-pdf", "pisah-pdf", "atur-halaman-pdf"].includes(request.options.tool)) {
    runPageTask(id, request, send).then(() => send({ event: "finished" }));
    return id;
  }

  (async () => {
    for (const [file, path] of request.paths.entries()) {
      for (let fraction = 0; fraction < 1; fraction += 0.2) {
        if (cancelled.has(id)) break;
        send({ event: "progress", file, fraction });
        await new Promise((resolve) => setTimeout(resolve, step));
      }
      if (cancelled.has(id)) {
        send({ event: "fileDone", file, outcome: { status: "failed", error: { code: "cancelled" } } });
        continue;
      }
      const size = sizeOf(path);
      const { tool, to, size: photoSize, maxSide } = request.options;
      const side = maxSide ?? SIDES[photoSize ?? "medium"];
      if (
        (to && kindOf(path) === (to === "jpeg" ? "jpg" : "png")) ||
        (tool === "ubah-ukuran-foto" && longestSideOf(path) <= side)
      ) {
        send({ event: "fileDone", file, outcome: { status: "failed", error: { code: "unchanged" } } });
        continue;
      }
      if (has("fail") && file === 1) {
        send({
          event: "fileDone",
          file,
          outcome: { status: "failed", error: { code: "size-limit-unreachable", best: Math.round(size * 0.4) } },
        });
        continue;
      }
      const dot = path.lastIndexOf(".");
      const extension = to ? (to === "jpeg" ? ".jpg" : ".png") : path.slice(dot);
      send({
        event: "fileDone",
        file,
        outcome: {
          status: "done",
          result: {
            inputPath: path,
            outputPath: `${path.slice(0, dot)} (${suffix})${extension}`,
            originalBytes: size,
            compressedBytes: Math.round(size * 0.12),
            ...(has("pages") && kindOf(path) === "pdf" ? pagesOf(path, size) : {}),
          },
        },
      });
    }
    send({ event: "finished" });
  })();
  return id;
}

export function installDevMocks() {
  mockIPC((cmd, args) => {
    switch (cmd) {
      case "list_tools":
        return TOOLS;
      case "list_history":
        return has("recent") ? sampleHistory() : [];
      case "clear_history":
        return null;
      case "get_settings":
        return { ...appSettings };
      case "set_keep_history":
        appSettings.keepHistory = (args as { keep: boolean }).keep;
        return { ...appSettings };
      case "set_results_folder":
        appSettings.resultsFolder = (args as { folder: string | null }).folder;
        return { ...appSettings };
      case "set_check_updates":
        appSettings.checkUpdates = (args as { check: boolean }).check;
        return { ...appSettings };
      case "check_update":
        return has("update") && appSettings.checkUpdates ? { version: "0.3.0", canInstall: !has("updateDeb") } : null;
      case "install_update":
        // Never restarts here; fails after a moment so the retry path shows.
        return new Promise((_, reject) => setTimeout(() => reject({ code: "update-failed" }), 1500));
      case "recent_limits":
        return has("recent") ? [...recentLimits] : [];
      case "list_presets": {
        const { tool } = args as { tool: string };
        return presets.filter((preset) => preset.tool === tool);
      }
      case "save_preset": {
        const { tool, name, options } = args as { tool: string; name: string; options: object };
        const preset = { id: String(Date.now()), tool, name: name.trim(), options };
        presets.push(preset);
        return preset;
      }
      case "rename_preset": {
        const { id, name } = args as { id: string; name: string };
        const preset = presets.find((p) => p.id === id);
        if (preset) preset.name = name.trim();
        return null;
      }
      case "remove_preset": {
        const { id } = args as { id: string };
        presets.splice(0, Infinity, ...presets.filter((p) => p.id !== id));
        return null;
      }
      case "recent_tools":
        return has("recent") ? ["kompres-pdf", "kompres-foto"] : [];
      case "match_files": {
        const paths = (args as { paths: string[] }).paths;
        if (paths.every(isPdf))
          return {
            status: "fits",
            kinds: ["pdf"],
            tools: paths.length > 1 ? ["kompres-pdf", "gabung-pdf", "pisah-pdf"] : ["kompres-pdf", "pisah-pdf", "atur-halaman-pdf"],
          };
        if (paths.some(isPdf)) return { status: "mixed" };
        return { status: "fits", kinds: ["jpg"], tools: ["kompres-foto", "ubah-format-foto", "ubah-ukuran-foto"] };
      }
      case "file_info":
        return (args as { paths: string[] }).paths.map((path) => ({
          size: sizeOf(path),
          kind: kindOf(path),
          pages: isPdf(path) ? pageCountOf(path) : null,
          longestSide: isPdf(path) ? null : longestSideOf(path),
        }));
      case "thumbnail": {
        const { path, size } = args as { path: string; size: number };
        return isPdf(path) ? new ArrayBuffer(0) : fakeThumbnail(path, size);
      }
      case "page_thumbnail": {
        const { path, page, size } = args as { path: string; page: number; size: number };
        return fakePage(path, page, size);
      }
      case "start_task":
        return runTask(args as unknown as TaskArgs);
      case "cancel_task":
        cancelled.add((args as { id: number }).id);
        return null;
      default:
        return null;
    }
  });
}
