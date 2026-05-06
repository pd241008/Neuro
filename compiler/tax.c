int main() {
    float income, tax;
    scanf("%f", &income);

    if(income <= 250000) 
        tax = 0;
    else if(income <= 500000) 
        tax = income * 0.05;
    else if(income <= 1000000) 
        tax = income * 0.20;
    else 
        tax = income * 0.30;

    printf("Tax = %.2f", tax);
    return 0;
}