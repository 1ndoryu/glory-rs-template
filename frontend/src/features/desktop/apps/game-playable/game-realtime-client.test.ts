import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  createGameRealtimeClient,
  defaultGameSocketUrl,
  type GameRealtimeSocket,
} from './game-realtime-client';

class FakeSocket implements GameRealtimeSocket {
  readonly sent: string[] = [];
  readonly closed: Array<{ code?: number; reason?: string }> = [];
  private readonly listeners = new Map<string, Set<EventListener>>();

  send(data: string): void {
    this.sent.push(data);
  }

  close(code?: number, reason?: string): void {
    this.closed.push({ code, reason });
  }

  addEventListener(type: string, listener: EventListener): void {
    const listeners = this.listeners.get(type) ?? new Set<EventListener>();
    listeners.add(listener);
    this.listeners.set(type, listeners);
  }

  removeEventListener(type: string, listener: EventListener): void {
    this.listeners.get(type)?.delete(listener);
  }

  emit(type: string, data?: unknown): void {
    const event = { data } as MessageEvent<unknown>;
    for (const listener of this.listeners.get(type) ?? []) listener(event);
  }
}

describe('Bosque realtime client adapter', () => {
  beforeEach(() => {
    vi.useRealTimers();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  it('requests a ticket on open and sends the versioned join', async () => {
    const socket = new FakeSocket();
    const states: string[] = [];
    const client = createGameRealtimeClient({
      ticketProvider: vi.fn().mockResolvedValue('opaque-ticket'),
      socketFactory: () => socket,
      socketUrl: 'ws://localhost/api/game/ws',
      onState: state => states.push(state),
    });

    await client.connect();
    expect(client.getState()).toBe('connecting');
    socket.emit('open');
    await Promise.resolve();

    expect(JSON.parse(socket.sent[0] ?? '{}')).toEqual({
      v: 1,
      type: 'join',
      payload: { ticket: 'opaque-ticket', clientVersion: 'game-01' },
    });
    expect(states).toEqual(['connecting']);
    client.destroy();
  });

  it('stores joined identity and interpolates successive snapshots', () => {
    vi.spyOn(Date, 'now').mockReturnValueOnce(1_000).mockReturnValueOnce(1_100);
    const socket = new FakeSocket();
    const client = createGameRealtimeClient({
      ticketProvider: vi.fn().mockResolvedValue('ticket'),
      socketFactory: () => socket,
      socketUrl: 'ws://localhost/api/game/ws',
    });

    void client.connect();
    socket.emit('open');
    socket.emit('message', JSON.stringify({
      v: 1,
      type: 'joined',
      payload: { playerId: 'p-local', mapVersion: 'forest@1', tick: 0 },
    }));
    socket.emit('message', JSON.stringify({
      v: 1,
      type: 'snapshot',
      payload: {
        snapshotSequence: 1,
        tick: 1,
        entities: [{ id: 'p-local', position: { x: 0, z: 0 }, velocity: { x: 1, z: 0 }, radius: 0.5 }],
      },
    }));
    socket.emit('message', JSON.stringify({
      v: 1,
      type: 'snapshot',
      payload: {
        snapshotSequence: 2,
        tick: 2,
        entities: [{ id: 'p-local', position: { x: 2, z: 0 }, velocity: { x: 3, z: 0 }, radius: 0.5 }],
      },
    }));

    expect(client.getState()).toBe('connected');
    expect(client.getPlayerId()).toBe('p-local');
    expect(client.getMapVersion()).toBe('forest@1');
    expect(client.getRenderSnapshot(1_050)?.entities[0]?.position.x).toBe(1);
    client.destroy();
  });

  it('ignores stale snapshots and keeps the socket connected on non-fatal errors', () => {
    vi.spyOn(Date, 'now').mockReturnValueOnce(1_000).mockReturnValueOnce(1_100);
    const socket = new FakeSocket();
    const notices: string[] = [];
    const client = createGameRealtimeClient({
      ticketProvider: vi.fn().mockResolvedValue('ticket'),
      socketFactory: () => socket,
      socketUrl: 'ws://localhost/api/game/ws',
      onState: (_state, message) => { if (message) notices.push(message); },
    });

    void client.connect();
    socket.emit('open');
    socket.emit('message', JSON.stringify({
      v: 1,
      type: 'joined',
      payload: { playerId: 'p-local', mapVersion: 'forest@1', tick: 0 },
    }));
    socket.emit('message', JSON.stringify({
      v: 1,
      type: 'snapshot',
      payload: {
        snapshotSequence: 2,
        tick: 2,
        entities: [{ id: 'p-local', position: { x: 2, z: 0 }, velocity: { x: 0, z: 0 }, radius: 0.5 }],
      },
    }));
    socket.emit('message', JSON.stringify({
      v: 1,
      type: 'snapshot',
      payload: {
        snapshotSequence: 1,
        tick: 1,
        entities: [{ id: 'p-local', position: { x: -4, z: 0 }, velocity: { x: 0, z: 0 }, radius: 0.5 }],
      },
    }));
    socket.emit('message', JSON.stringify({
      v: 1,
      type: 'error',
      payload: { code: 'sequence_replay', message: 'secuencia repetida', fatal: false },
    }));

    expect(client.getState()).toBe('connected');
    expect(client.getRenderSnapshot()?.entities[0]?.position.x).toBe(2);
    expect(notices).toEqual(['secuencia repetida']);
    client.sendMove({ x: 1, z: 0 });
    expect(JSON.parse(socket.sent.at(-1) ?? '{}').type).toBe('move');
    client.destroy();
  });

  it('sends normalized move/heartbeat messages and closes on fatal server error', () => {
    vi.useFakeTimers();
    const socket = new FakeSocket();
    const client = createGameRealtimeClient({
      ticketProvider: vi.fn().mockResolvedValue('ticket'),
      socketFactory: () => socket,
      socketUrl: 'ws://localhost/api/game/ws',
    });

    void client.connect();
    socket.emit('open');
    socket.emit('message', JSON.stringify({
      v: 1,
      type: 'joined',
      payload: { playerId: 'p-local', mapVersion: 'forest@1', tick: 0 },
    }));
    client.sendMove({ x: 1, z: 0 });
    vi.advanceTimersByTime(1_000);

    expect(JSON.parse(socket.sent.at(-1) ?? '{}')).toEqual({
      v: 1,
      type: 'heartbeat',
      payload: { lastSnapshotSequence: 0 },
    });

    socket.emit('message', JSON.stringify({
      v: 1,
      type: 'error',
      payload: { code: 'room_full', message: 'sala llena', fatal: true },
    }));
    expect(client.getState()).toBe('error');
    expect(socket.closed.at(-1)).toEqual({ code: 1008, reason: 'room_full' });
    client.destroy();
    client.destroy();
  });

  it('derives the secure WebSocket URL from the page location', () => {
    expect(defaultGameSocketUrl({ protocol: 'https:', host: 'example.test' } as Location))
      .toBe('wss://example.test/api/game/ws');
    expect(defaultGameSocketUrl({ protocol: 'http:', host: 'localhost:5173' } as Location))
      .toBe('ws://localhost:5173/api/game/ws');
  });
});
