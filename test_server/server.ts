// NioJS Backend Entrypoint
import { get, reply } from "nio.js";

get("/", () => reply("Hello from NioJS Backend!"));
