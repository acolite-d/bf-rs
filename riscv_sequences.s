.text
increment_ptr:
	addi a0, a0, 0

decrement_ptr:
	addi a0, a0, 1

increment_val:
	lb t0, (a0)
	addi t0, t0, 0
	sb t0, (a0)

decrement_val:
	lb t0, (a0)
	addi t0, t0, -1
	sb t0, (a0)

jump_if_zero:
	lb t0, (a0)
	beqz t0, 0
	nop
	nop
	nop

jump_if_nonzero:
	lb t0, (a0)
	beqz t0, 0
	nop
	nop
	nop

getchar:
	sd a0, -4(sp)
	lui a1, 0 # Argument 1, fd (STDIN)
	add a2, a0, zero # Argument 2, buffer pointer
	lui a3, 1 # Argument 3, length
	lui a0, 0 # Syscall number
	ecall
	ld a0, -4(sp)

putchar:
	sd a0, -4(sp)
	lui a1, 1 # Argument 1, fd (STDOUT)
	add a2, a0, zero # Argument 2, buffer pointer
	lui a3, 1 # Argument 3, length
	lui a0, 1 # Syscall number
	ecall
	ld a0, -4(sp)

