import { describe, expect, it } from 'vitest';
import { DEFAULT_RELEASE } from './default-release';
import { withLocalPrototypeNodes } from './local-development-release';
import type { WorkspaceTree } from './types';

describe('local development release compatibility', () => {
  it('restores the visual forest entries when an old local release omits them', () => {
    const oldRelease: WorkspaceTree = {
      version: 2,
      nodes: { about: DEFAULT_RELEASE.nodes.about },
    };

    const resolved = withLocalPrototypeNodes(oldRelease);

    expect(resolved.nodes.game?.refId).toBe('game');
    expect(resolved.nodes.game3d?.refId).toBe('game-3d');
    expect(resolved.version).toBe(2);
  });

  it('does not overwrite an entry already organized by the release', () => {
    const organized = {
      ...DEFAULT_RELEASE.nodes.game3d,
      label: 'Bosque 3D organizado',
      position: { col: 7, row: 4 },
    };
    const release: WorkspaceTree = {
      version: 3,
      nodes: { game3d: organized },
    };

    const resolved = withLocalPrototypeNodes(release);

    expect(resolved.nodes.game3d).toEqual(organized);
  });
});
