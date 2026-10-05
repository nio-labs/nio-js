/** nio-js 0.1 preview. Add this file to your tsconfig's include list. */
declare module 'nio.js' {
  export interface RequestContext {
    method: string;
    url: string;
    headers: Headers;
    query: Record<string, string | undefined>;
    searchParams: URLSearchParams;
    params: Record<string, string>;
    requestId: string;
    text(): Promise<string>;
    json(): Promise<unknown>;
    blob(): Promise<Blob>;
    formData(): Promise<FormData>;
  }
  export interface ReplyOptions { status?: number; headers?: Record<string, string> }
  export interface ExplicitReply { readonly __nioReply: true; body: unknown; options: ReplyOptions }
  export type Json = string | number | boolean | null | Json[] | { [key: string]: Json };
  export type Body = string | Blob | Json[] | { [key: string]: unknown };
  export type Result = Body | Response | ExplicitReply;
  export type Handler = (request: RequestContext) => Result | Promise<Result>;
  export function get(path: string, handler: Handler | string | Blob | ExplicitReply): void;
  export function post(path: string, handler: Handler | string | Blob | ExplicitReply): void;
  export function reply(body: unknown, options?: ReplyOptions): ExplicitReply;
  export function asset(name: string): Blob;
  export function redirect(location: string, status?: number): ExplicitReply;
}
