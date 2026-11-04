import type { Engine, Event, Snapshot } from "../../engines/contract";
import type { FactoryInput } from "../../engines/factory";
import { createRustFactory } from "../../engines/rust-factory";
import { factoryScenario } from "../../scenarios/factory";
import { compareFactoryScenario } from "../../traces/factory";

export type FactoryView = {
  instances: { name: string; input: FactoryInput; snapshot: Snapshot }[];
  comparison?: ReturnType<typeof compareFactoryScenario>;
  error?: string;
};

export class FactorySession {
  private readonly engines = new Map<string, Engine>();
  private readonly listeners = new Set<() => void>();
  private view: FactoryView;

  constructor() {
    const factory = createRustFactory();
    try {
      for (const { name, input } of factoryScenario.instances)
        this.engines.set(name, factory.create(input));
    } catch (error) {
      for (const engine of this.engines.values()) engine.dispose();
      throw error;
    } finally {
      factory.dispose();
    }
    this.view = { instances: this.snapshots() };
    for (const engine of this.engines.values())
      engine.subscribe(() => this.update({ instances: this.snapshots() }));
  }

  getSnapshot = () => this.view;

  subscribe = (callback: () => void) => {
    this.listeners.add(callback);
    return () => this.listeners.delete(callback);
  };

  send = (name: string, event: Event) => {
    try {
      this.engines.get(name)!.send(event);
      this.update({ error: undefined });
    } catch (error) {
      this.update({
        error: error instanceof Error ? error.message : String(error),
      });
    }
  };

  compare = () => {
    try {
      this.update({
        comparison: compareFactoryScenario(factoryScenario),
        error: undefined,
      });
    } catch (error) {
      this.update({
        error: error instanceof Error ? error.message : String(error),
      });
    }
  };

  dispose = () => {
    for (const engine of this.engines.values()) engine.dispose();
    this.listeners.clear();
  };

  private snapshots() {
    return factoryScenario.instances.map(({ name, input }) => ({
      name,
      input,
      snapshot: this.engines.get(name)!.snapshot(),
    }));
  }

  private update(change: Partial<FactoryView>) {
    this.view = { ...this.view, ...change };
    for (const callback of this.listeners) callback();
  }
}
