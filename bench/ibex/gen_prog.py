import sys
def R(f7,rs2,rs1,f3,rd,op): return (f7<<25)|(rs2<<20)|(rs1<<15)|(f3<<12)|(rd<<7)|op
def I(imm,rs1,f3,rd,op): return ((imm&0xfff)<<20)|(rs1<<15)|(f3<<12)|(rd<<7)|op
def S(imm,rs2,rs1,f3,op):
    imm&=0xfff
    return ((imm>>5)<<25)|(rs2<<20)|(rs1<<15)|(f3<<12)|((imm&0x1f)<<7)|op
def B(imm,rs2,rs1,f3,op):
    imm&=0x1fff
    b12=(imm>>12)&1; b11=(imm>>11)&1; b10_5=(imm>>5)&0x3f; b4_1=(imm>>1)&0xf
    return (b12<<31)|(b10_5<<25)|(rs2<<20)|(rs1<<15)|(f3<<12)|(b4_1<<8)|(b11<<7)|op
def J(imm,rd,op):
    imm&=0x1fffff
    b20=(imm>>20)&1; b10_1=(imm>>1)&0x3ff; b11=(imm>>11)&1; b19_12=(imm>>12)&0xff
    return (b20<<31)|(b10_1<<21)|(b11<<20)|(b19_12<<12)|(rd<<7)|op
OPI=0x13; OPR=0x33; LD=0x03; ST=0x23; BR=0x63; JAL=0x6f
prog=[]
# 0x80
prog.append(I(0x400,0,0b010,3,LD))      # lw x3, 1024(x0)   N
prog.append(I(0,0,0b000,1,OPI))         # addi x1,x0,0      acc
prog.append(I(0,0,0b000,2,OPI))         # addi x2,x0,0      i
LOOP=0x8c
prog.append(R(0,2,1,0b000,1,OPR))       # add x1,x1,x2
prog.append(I(3,2,0b001,4,OPI))         # slli x4,x2,3
prog.append(R(0,4,1,0b100,1,OPR))       # xor x1,x1,x4
prog.append(R(1,2,2,0b000,5,OPR))       # mul x5,x2,x2
prog.append(R(0,5,1,0b000,1,OPR))       # add x1,x1,x5
prog.append(I(7,1,0b101,6,OPI))         # srli x6,x1,7
prog.append(R(0,6,1,0b000,1,OPR))       # add x1,x1,x6
prog.append(I(1,2,0b000,2,OPI))         # addi x2,x2,1
prog.append(S(0x700,1,0,0b010,ST))      # sw x1, 1792(x0)
prog.append(B(LOOP-0xb0,3,2,0b100,BR))  # blt x2,x3,LOOP
prog.append(S(0x404,1,0,0b010,ST))      # sw x1, 1028(x0)  sentinel
prog.append(J(0,0,JAL))                 # jal x0,0  spin
for i,w in enumerate(prog):
    print("    mem[13'h%03x] = 32'h%08x;" % ((0x80+4*i)>>2, w))
