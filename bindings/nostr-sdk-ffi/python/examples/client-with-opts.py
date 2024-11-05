import asyncio
from nostr_sdk import Keys, Client, Options, EventBuilder, Connection, ConnectionTarget, init_logger, LogLevel
from datetime import timedelta


async def main():
    init_logger(LogLevel.INFO)

    # Configure client to use proxy for `.onion` relays
    connection = Connection().addr("127.0.0.1:9050").target(ConnectionTarget.ONION)
    opts = (Options()
            .connection_timeout(timedelta(seconds=60))
            .connection(connection))
    client = Client.with_opts(None, opts)

    await client.add_relay("wss://relay.damus.io")
    await client.add_relay("ws://oxtrdevav64z64yb7x6rjg4ntzqjhedm5b5zjqulugknhzr46ny2qbad.onion")
    await client.connect()

    keys = Keys.generate()
    print(keys.public_key().to_bech32())

    event = EventBuilder.text_note("Hello from rust-nostr Python bindings!", []).sign_with_keys(keys)
    event_id = await client.send_event(event)
    print("Event sent:")
    print(f" hex:    {event_id.to_hex()}")
    print(f" bech32: {event_id.to_bech32()}")


if __name__ == '__main__':
    asyncio.run(main())