// In-memory PostgreSQL for backend tests, using PGlite (Postgres compiled to
// WASM) exposed over the Postgres wire protocol via a TCP socket. SeaORM/sqlx
// connect to it exactly like a real Postgres server — no Docker, no system
// Postgres install required.
//
// Usage:  node server.mjs [--host 127.0.0.1] [--port 5432]
// Stop:   SIGINT / SIGTERM (the runner script handles teardown).
import { PGlite } from '@electric-sql/pglite'
import { PGLiteSocketServer } from '@electric-sql/pglite-socket'

const args = process.argv.slice(2)
const opt = (name, def) => {
  const i = args.indexOf(`--${name}`)
  return i !== -1 && args[i + 1] ? args[i + 1] : def
}

const host = opt('host', process.env.PGLITE_HOST ?? '127.0.0.1')
const port = Number(opt('port', process.env.PGLITE_PORT ?? '5432'))
// PGLiteSocketServer defaults to maxConnections=1 (NO concurrent connections),
// which loco's test boot violates: it recreates+migrates on one connection
// while the SeaORM pool opens another, and parallel test threads each boot an
// app. With the default the server resets the extra sockets — every test after
// the first fails with `Connection reset by peer` / a corrupted SSLRequest
// reply. Multiplex over the single PGlite instance instead.
const maxConnections = Number(
  opt('max-connections', process.env.PGLITE_MAX_CONNECTIONS ?? '100'),
)

// `dataDir` omitted => purely in-memory; every boot starts from a clean DB.
const db = await PGlite.create()
await db.waitReady

const server = new PGLiteSocketServer({ db, host, port, maxConnections })

// pglite-socket multiplexes every TCP connection onto ONE shared PGlite
// session, so server-prepared statements are global — they are NOT scoped per
// connection like a real Postgres backend. sqlx names its prepared statements
// per-connection starting at `sqlx_s_1`, and (regardless of
// `statement-cache-capacity`) always uses a NAMED statement for persistent
// queries. So the connection loco opens for test N+1 tries to `PREPARE
// sqlx_s_1` again and hits `prepared statement "sqlx_s_1" already exists`
// (42P05) — the leftover from test N is still registered on the shared session.
//
// Loco's test boot recreates the schema (`dangerously_recreate`), so tests MUST
// run serially anyway (parallel boots would clobber each other's tables); the
// suite is `--test-threads=1`. That means when a new client connects, the
// previous boot's connection is already idle/finished, so it is safe to wipe
// all prepared statements. `db.exec` and the socket handler both go through
// PGlite's single `runExclusive` mutex, and this fires on connect (before sqlx
// finishes its startup handshake), so the DEALLOCATE wins the lock before the
// first PREPARE.
server.addEventListener('connection', () => {
  db.exec('DEALLOCATE ALL').catch((err) => {
    console.warn('[pglite] DEALLOCATE ALL on connect failed:', err?.message ?? err)
  })
})

await server.start()
console.log(
  `[pglite] in-memory Postgres listening on ${host}:${port} (maxConnections=${maxConnections})`,
)

const shutdown = async () => {
  try {
    await server.stop()
    await db.close()
  } finally {
    process.exit(0)
  }
}
process.on('SIGINT', shutdown)
process.on('SIGTERM', shutdown)
