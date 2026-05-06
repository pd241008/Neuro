; --- Generated Assembly from AST ---
main:
  push rbp
  mov rbp, rsp
  sub rsp, 16

  ; scanf("%f", &income);
  call scanf

  ; if(income <= 250000)
  movss xmm0, DWORD PTR [rbp-4]
  ucomiss xmm0, DWORD PTR .LC_CONST_250000[rip]
  ja .L_FALSE_1
  ; tax =
  pxor xmm0, xmm0
  movss DWORD PTR [rbp-8], xmm0

  jmp .L_END_2

.L_FALSE_1:
  ; if(income <= 500000)
  movss xmm0, DWORD PTR [rbp-4]
  ucomiss xmm0, DWORD PTR .LC_CONST_500000[rip]
  ja .L_FALSE_3
  ; tax =
  movss xmm0, DWORD PTR [rbp-4] ; Load income
  mulss xmm0, DWORD PTR .LC_CONST_0.05[rip]
  movss DWORD PTR [rbp-8], xmm0

  jmp .L_END_4

.L_FALSE_3:
  ; if(income <= 1000000)
  movss xmm0, DWORD PTR [rbp-4]
  ucomiss xmm0, DWORD PTR .LC_CONST_1000000[rip]
  ja .L_FALSE_5
  ; tax =
  movss xmm0, DWORD PTR [rbp-4] ; Load income
  mulss xmm0, DWORD PTR .LC_CONST_0.20[rip]
  movss DWORD PTR [rbp-8], xmm0

  jmp .L_END_6

.L_FALSE_5:
  ; tax =
  movss xmm0, DWORD PTR [rbp-4] ; Load income
  mulss xmm0, DWORD PTR .LC_CONST_0.30[rip]
  movss DWORD PTR [rbp-8], xmm0

.L_END_6:
.L_END_4:
.L_END_2:
  ; printf("Tax = %f", tax);
  movss xmm0, DWORD PTR [rbp-8] ; Assuming tax is rbp-8
  call printf

  ; return 0;
  mov eax, 0
  leave
  ret
