import { api } from "$lib/api";
import type { CreateInstance, Instance, UpdateInstance } from "$lib/types";

/// Reactive store (Svelte 5 runes) holding all instances in memory.
class InstancesStore {
  instances = $state<Instance[]>([]);
  loading = $state(false);
  loaded = $state(false);
  error = $state<string | null>(null);

  async refresh() {
    this.loading = true;
    this.error = null;
    try {
      this.instances = await api.listInstances();
      this.loaded = true;
    } catch (error) {
      this.error = String(error);
    } finally {
      this.loading = false;
    }
  }

  /** Load once; subsequent calls are no-ops unless forced via refresh(). */
  async ensureLoaded() {
    if (!this.loaded && !this.loading) await this.refresh();
  }

  get(id: string): Instance | undefined {
    return this.instances.find((instance) => instance.id === id);
  }

  async create(payload: CreateInstance): Promise<Instance> {
    const created = await api.createInstance(payload);
    await this.refresh();
    return created;
  }

  async update(id: string, patch: UpdateInstance): Promise<Instance> {
    const updated = await api.updateInstance(id, patch);
    await this.refresh();
    return updated;
  }

  async remove(id: string): Promise<void> {
    await api.deleteInstance(id);
    await this.refresh();
  }

  /** Set a custom icon (data URI). */
  async setIcon(id: string, dataUri: string): Promise<void> {
    await this.update(id, { icon: dataUri });
  }

  /** Clear the custom icon, falling back to the generated default.
      (The backend treats an empty string as "reset".) */
  async resetIcon(id: string): Promise<void> {
    await this.update(id, { icon: "" });
  }
}

export const instancesStore = new InstancesStore();
