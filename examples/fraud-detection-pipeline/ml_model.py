def predict_fraud(amount, account_age_days):
    amount = float(amount)
    account_age_days = float(account_age_days)
    
    risk_score = 0.0
    if amount > 10000.0:
        risk_score += 0.5
    if account_age_days < 30.0:
        risk_score += 0.3
        
    if amount > 50000.0 and account_age_days < 7.0:
        risk_score += 0.8
        
    return min(1.0, risk_score)
