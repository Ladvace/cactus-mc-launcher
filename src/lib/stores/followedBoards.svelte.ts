import { readJson, writeJson } from "$lib/storage";

const KEY = "cactus:followedBoards";

function load(): string[] {
  return readJson<string[]>(KEY, []);
}

class FollowedBoards {
  handles = $state<string[]>(load());

  isFollowing(handle: string): boolean {
    return this.handles.includes(handle);
  }

  follow(handle: string) {
    if (this.handles.includes(handle)) return;
    this.handles = [...this.handles, handle];
    this.save();
  }

  unfollow(handle: string) {
    this.handles = this.handles.filter((existing) => existing !== handle);
    this.save();
  }

  private save() {
    writeJson(KEY, this.handles);
  }
}

export const followedBoards = new FollowedBoards();
