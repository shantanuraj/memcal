# memcal

`memcal` is an iCal compatible server. It accepts existing iCal urls and
responds with the same data. With one important distinction: it parses the
source data and stores them in a datastore.
This is useful for iCal feeds that remove events after they have passed.

## Pre-requisites

You will need `cargo` to build the project.
`sqlite3` is required to interact with the sqlite database.

### Create initial database

```bash
mkdir -p data
touch data/memcal.db
```

## Building

You can build the project into a binary using `cargo`.

```bash
cargo build
```

It will create a debug binary at `target/debug/memcal`.

```bash
target/debug/memcal
```

You can also build a release binary.

```bash
cargo build --release
```

It will create a release binary at `target/release/memcal`.

```bash
target/release/memcal
```

## Running

Run the binary to start the server, or use `cargo run`.

```bash
cargo run
```

The server will start at `http://localhost:8080`.

To change the port you can use the `PORT` environment variable.
By default the server will use port 8080.

```bash
PORT=3000 cargo run
```

To change the database path you can use the `DATABASE_URL` environment variable.
The default database path is `sqlite:data/memcal.db`.

```bash
DATABASE_URL=sqlite:/var/data/memcal.db cargo run
```

## Architecture

The primary interaction with `memcal` is through the REST API.
The API is a simple CRUD interface that allows to manage iCal feeds.

- `POST /feed` - Add a new iCal feed
- `GET /feed/:id` - Get a memorized iCal feed
- `DELETE /feed/:id/:manage_token` - Remove a memorized iCal feed
- `DELETE /feed/:id/:event_id/:manage_token` - Remove a single event from a memorized iCal feed

The syncing is independent of the API. It's a background process that runs
every 5 minutes. It fetches the iCal feeds and updates the datastore.

We also expose a web interface that allows to manage the feeds.

- You can add new feeds.
- Delete existing feeds.
- Delete events from a feed.

There's no plans for authentication or authorization at the moment.

### Adding a new feed

To add a new feed you can use the API or the web interface.

```bash
curl -H "content-type: application/json" \
    -d '{"url": "https://example.com/feed.ics"}' \
    http://localhost:8080/feed

# or using web forms
curl -i -H "content-type: application/x-www-form-urlencoded" \
    -d "url=https://example.com/feed.ics" \
    http://localhost:8080/feed
```

This will respond with the url of memorized feed.

```js
{
  "url": "http://localhost:8080/feed/<feed_id>",
  "manage_token": "<manage_token>",
  "manage_url": "http://localhost:8080/feed/feed_id/<manage_token>"
}
// Web forms will redirect to the manage feed page
// /feed/<feed_id>/<manage_token>
// HTTP/1.1 303 See Other
// location: /feed/<feed_id>/<manage_token>
```

The `feed_id` is a short alphanumeric code that identifies the feed.
The `manage_token` is a token that allows to manage the feed. With this token
you can delete the feed.
The `manage_url` is the url that allows to delete the feed using the web UI.

### Getting a feed

To get a memorized feed you can use the feed url you got when adding the feed.

```bash
curl http://localhost:8080/feed/<feed_id>
```

This will respond with the iCal data that can be used in any iCal compatible
client.

### Deleting a feed

To delete a memorized feed you can use the feed url you got when adding the feed.

```bash
curl -X DELETE \
    -H "content-type: application/json" -d '{}' \
    http://localhost:8080/feed/<feed_id>/<manage_token>

# or to support web forms that don't support DELETE

curl -X POST \
    -H "content-type: application/x-www-form-urlencoded" \
    -d "_method=DELETE" \
    http://localhost:8080/feed/<feed_id>/<manage_token>
```

This will respond with a 204 status code if the feed was deleted successfully.

### Deleting an event

To delete an event from a memorized feed you can use the feed url you got when
adding the feed.

```bash
curl -X DELETE \
    -H "content-type: application/json" -d '{}' \
    http://localhost:8080/feed/<feed_id>/<event_id>/<manage_token>

# or to support web forms that don't support DELETE

curl -X POST \
    -H "content-type: application/x-www-form-urlencoded" \
    -d "_method=DELETE" \
    http://localhost:8080/feed/<feed_id>/<event_id>/<manage_token>
```

This will respond with a 204 status code if the event was deleted successfully.

### Web interface

The web interface is available at `http://localhost:8080`.

The web interface is available at `http://localhost:8080`.
It allows to add new feeds and manage existing feeds, and events.
It will redirect you to the feed management page at
`http://localhost:8080/feed/<feed_id>/<manage_token>`
This shows the iCal url and options to delete the feed or individual events.

## Future

- At some point it might be worth to explore url parameters that allow smart
  features like filtering, sorting, etc.
  eg. `?since=2020-01-01` or `?since=last_year`
