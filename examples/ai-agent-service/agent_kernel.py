import json

def analyze_intent(query: str) -> str:
    """Analyze agent query intent and categorize it."""
    q = query.lower()
    if any(k in q for k in ["fix", "bug", "error", "traceback", "crash", "fail"]):
        return "debugging"
    elif any(k in q for k in ["build", "compile", "bundle", "test", "release"]):
        return "devops"
    elif any(k in q for k in ["search", "find", "grep", "where", "list"]):
        return "retrieval"
    elif any(k in q for k in ["refactor", "optimize", "speed", "benchmark"]):
        return "performance"
    else:
        return "general_reasoning"

def score_context(doc_text: str, keywords_json: str) -> float:
    """Score relevance of context document against query keywords."""
    keywords = json.loads(keywords_json)
    if not keywords:
        return 0.0
    text = doc_text.lower()
    matches = sum(1 for kw in keywords if kw.lower() in text)
    return round(matches / len(keywords), 3)

def summarize_task(title: str, steps_count: int, priority: str) -> dict:
    """Format and validate an agent task summary."""
    return {
        "title": title.strip(),
        "steps_count": steps_count,
        "priority": priority.upper(),
        "status": "ready_for_execution"
    }
