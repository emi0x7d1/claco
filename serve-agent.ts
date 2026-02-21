import { copy } from "https://deno.land/std@0.224.0/io/mod.ts";

const BINARY_PATH = "target/release/claco-acp-agent";
const ARCHIVE_NAME = "claco-agent-darwin-aarch64.tar.gz";
const PORT = 8003;

async function createArchive() {
    console.log(`Creating archive ${ARCHIVE_NAME}...`);

    // Create a temporary directory or just tar it from target/release?
    // Zed expects the binary to be at the path specified by 'cmd' in the archive.
    // In extension.toml we set cmd = "claco-acp-agent".
    // So the archive should contain claco-acp-agent at the root.

    const cmd = new Deno.Command("tar", {
        args: [
            "-czf",
            ARCHIVE_NAME,
            "-C",
            "target/debug",
            "claco-acp-agent",
            "wrapper.sh",
        ],
    });

    const { success, stderr } = await cmd.output();
    if (!success) {
        console.error("Failed to create archive:");
        console.error(new TextDecoder().decode(stderr));
        Deno.exit(1);
    }
    console.log("Archive created successfully.");
}

async function serve() {
    console.log(
        `Serving ${ARCHIVE_NAME} at http://localhost:${PORT}/${ARCHIVE_NAME}`,
    );

    Deno.serve({ port: PORT }, async (req) => {
        const url = new URL(req.url);
        if (url.pathname.endsWith(`/${ARCHIVE_NAME}`)) {
            try {
                const file = await Deno.open(ARCHIVE_NAME, { read: true });
                return new Response(file.readable, {
                    headers: {
                        "content-type": "application/gzip",
                        "content-disposition":
                            `attachment; filename="${ARCHIVE_NAME}"`,
                    },
                });
            } catch (e) {
                return new Response("File not found", { status: 404 });
            }
        }
        return new Response("Not Found", { status: 404 });
    });
}

if (import.meta.main) {
    await createArchive();
    await serve();
}
