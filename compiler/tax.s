call scanf, "%f", &income

    if income > 250000.0 goto .L_FALSE_1
    tax = 0.0
    goto .L_END_ALL

.L_FALSE_1:
    if income > 500000.0 goto .L_FALSE_2
    tax = income * 0.05
    goto .L_END_ALL

.L_FALSE_2:
    if income > 1000000.0 goto .L_FALSE_3
    tax = income * 0.20
    goto .L_END_ALL

.L_FALSE_3:
    tax = income * 0.30

.L_END_ALL:
    call printf, "Tax = %.2f", tax