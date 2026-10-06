import { get, post, reply } from 'nio.js';
import { analyze_intent, score_context, summarize_task } from './agent_kernel.py';

// Autonomous AI agent tool dispatcher and intent service
get('/agent/health', () => ({
  status: 'online',
  capabilities: ['intent_classification', 'context_scoring', 'task_planning'],
  engine: 'nio-js-hybrid-python'
}));

// Query classification using in-process Python AI
get('/agent/classify', ({ query }) => {
  const prompt = query.prompt || 'Help me debug this failing compile error';
  const intent = analyze_intent(prompt);
  
  return {
    prompt,
    intent,
    confidence: 0.94,
    routed_tool: intent === 'debugging' ? 'code_debugger' : 'knowledge_retriever'
  };
});

// Context relevance scoring for Retrieval Augmented Generation (RAG)
post('/agent/score-context', async ({ json }) => {
  const body = await json();
  if (!body.document || !Array.isArray(body.keywords)) {
    return reply(
      { error: 'Invalid payload: "document" (string) and "keywords" (string array) are required.' },
      { status: 400 }
    );
  }

  const score = score_context(body.document, JSON.stringify(body.keywords));
  return {
    document_length: body.document.length,
    keywords_matched: body.keywords,
    relevance_score: score,
    is_usable: score >= 0.5
  };
});

// Autonomous task planning
post('/agent/plan', async ({ json }) => {
  const body = await json();
  const title = body.title || 'Automated Refactoring Pipeline';
  const steps = Array.isArray(body.steps) ? body.steps.length : 3;
  const priority = body.priority || 'high';

  const plan = summarize_task(title, steps, priority);
  return reply({ success: true, plan }, { status: 200 });
});
