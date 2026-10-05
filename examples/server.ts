import { get, post, reply } from 'nio.js'

get('/', 'Hello World')
get('/health', () => ({ status: 'ok' }))
get('/greeting', ({ query }) => {
  const name: string = query.name ?? 'World'
  return `Hello ${name}`
})
get('/users/:id', async ({ params }) => ({ id: params.id }))
post('/echo', async ({ json }) => reply(await json(), { status: 201 }))
post('/upload', async ({ formData }) => {
  const file = (await formData()).get('file')
  if (!(file instanceof File)) return reply({ error: 'Choose a file' }, { status: 400 })
  return { name: file.name, size: file.size, content: await file.text() }
})
