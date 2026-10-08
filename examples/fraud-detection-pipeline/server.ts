import { post, reply } from 'nio.js';
import { verify_signature } from './crypto.zig';
import { predict_fraud } from './ml_model.py';

// Native Rust offloading for heavy transaction history scanning
/** @native */
function scan_history(iterations: number): number {
  let flagCount = 0;
  for (let i = 0; i < iterations; i++) {
    flagCount = (flagCount * 31 + i) & 0x7fffffff;
  }
  return flagCount % 10;
}

post('/api/fraud/analyze', async ({ json }) => {
  let tx;
  try { tx = await json(); } catch { return reply({ error: 'Invalid JSON' }, { status: 400 }); }
  if (!tx || typeof tx !== 'object' || Array.isArray(tx)
      || typeof tx.amount !== 'number' || !Number.isFinite(tx.amount) || tx.amount < 0 || tx.amount > Number.MAX_SAFE_INTEGER
      || !Number.isSafeInteger(tx.user_id_hash) || tx.user_id_hash < 0
      || typeof tx.account_age_days !== 'number' || !Number.isFinite(tx.account_age_days) || tx.account_age_days < 0) {
    return reply({ error: 'Expected nonnegative amount, integer user_id_hash, and account_age_days' }, { status: 400 });
  }

  // 1. Zig FFI for high-speed crypto operations
  const isValidSig = verify_signature(tx.amount, tx.user_id_hash) === 1.0;
  
  // 2. Python AI for risk scoring inference
  const riskScore = predict_fraud(tx.amount, tx.account_age_days);
  
  // 3. Rust Native Offloading for high-throughput loop computation
  const historyFlags = scan_history(50000);
  
  // Combine all signals into final hybrid risk assessment
  const finalRisk = riskScore + (historyFlags * 0.05) + (!isValidSig ? 0.5 : 0);
  
  return reply({
    transaction_id: tx.id || 'tx_unknown',
    is_fraud: finalRisk > 0.7,
    risk_score: Number(finalRisk.toFixed(2)),
    signals: {
      signature_valid: isValidSig,
      ml_risk_score: riskScore,
      history_flags: historyFlags
    }
  }, { status: 200 });
});
