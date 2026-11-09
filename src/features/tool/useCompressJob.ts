import { useCallback, useRef, useState } from "react";
import { fileName } from "@/features/compress/imageApi";
import {
  cancelTask,
  startTask,
  type TaskError,
  type TaskMessage,
  type TaskResult,
  type ToolOptions,
} from "./taskApi";

interface ItemBase {
  path: string;
  name: string;
  /** The options this file was last run with, so it can be tried again. */
  options: ToolOptions;
}

export type JobItem =
  /** "unchanged": already as asked, so no Result was written. */
  | (ItemBase & { status: "pending" | "cancelled" | "unchanged" })
  | (ItemBase & { status: "working"; fraction: number })
  | (ItemBase & { status: "done"; result: TaskResult })
  | (ItemBase & { status: "failed"; error: TaskError });

/** Runs one Task over a batch of files and tracks each file's status. */
export function useCompressJob() {
  const [items, setItems] = useState<JobItem[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const running = useRef(false);
  /** Set by `reset` during a Task: empty the list once the Task has stopped. */
  const discard = useRef(false);
  const taskId = useRef<number | null>(null);
  const itemsRef = useRef(items);
  itemsRef.current = items;

  const finish = useCallback(() => {
    running.current = false;
    taskId.current = null;
    setIsRunning(false);
    if (discard.current) {
      discard.current = false;
      setItems([]);
    }
  }, []);

  /** Run `options` over the items at `indexes`; the rest of the list stays as it is. */
  const run = useCallback(
    async (indexes: number[], options: ToolOptions) => {
      running.current = true;
      setIsRunning(true);

      const update = (index: number, change: (item: ItemBase) => JobItem) =>
        setItems((prev) => prev.map((old, i) => (i === index ? change(old) : old)));
      const paths = indexes.map((i) => itemsRef.current[i].path);

      const onMessage = (message: TaskMessage) => {
        switch (message.event) {
          case "progress":
            return update(indexes[message.file], ({ path, name }) => ({
              path,
              name,
              options,
              status: "working",
              fraction: message.fraction,
            }));
          case "fileDone": {
            const { outcome } = message;
            return update(indexes[message.file], ({ path, name }) => {
              const base = { path, name, options };
              if (outcome.status === "done") return { ...base, status: "done", result: outcome.result };
              if (outcome.error.code === "cancelled") return { ...base, status: "cancelled" };
              if (outcome.error.code === "unchanged") return { ...base, status: "unchanged" };
              return { ...base, status: "failed", error: outcome.error };
            });
          }
          case "finished":
            return finish();
        }
      };

      try {
        const id = await startTask({ options, paths }, onMessage);
        // A short Task may already be over by the time its id arrives.
        if (running.current) {
          taskId.current = id;
          if (discard.current) cancelTask(id);
        }
      } catch (error) {
        const failed = { code: (error as TaskError | undefined)?.code ?? "unexpected" };
        for (const index of indexes) {
          update(index, ({ path, name }) => ({ path, name, options, status: "failed", error: failed }));
        }
        finish();
      }
    },
    [finish],
  );

  const start = useCallback(
    (paths: string[], options: ToolOptions) => {
      // Ignore a new batch (e.g. from the menu) while one is still running.
      if (running.current || paths.length === 0) return;
      const fresh: JobItem[] = paths.map((path) => ({ path, name: fileName(path), options, status: "pending" }));
      itemsRef.current = fresh;
      setItems(fresh);
      run(
        paths.map((_, i) => i),
        options,
      );
    },
    [run],
  );

  /** Run one file again, with its last options or with `options`. */
  const retry = useCallback(
    (index: number, options?: ToolOptions) => {
      const item = itemsRef.current[index];
      if (running.current || !item) return;
      const next = options ?? item.options;
      itemsRef.current = itemsRef.current.map((old, i) =>
        i === index ? { path: old.path, name: old.name, options: next, status: "pending" } : old,
      );
      setItems(itemsRef.current);
      run([index], next);
    },
    [run],
  );

  /** Stops the running Task; finished Results stay. */
  const cancel = useCallback(() => {
    if (taskId.current !== null) cancelTask(taskId.current);
  }, []);

  /** Empties the list; a running Task is stopped first, and its finished Results stay. */
  const reset = useCallback(() => {
    if (running.current) {
      discard.current = true;
      if (taskId.current !== null) cancelTask(taskId.current);
    } else {
      setItems([]);
    }
  }, []);

  return { items, isRunning, start, retry, cancel, reset };
}

export type CompressJob = ReturnType<typeof useCompressJob>;
