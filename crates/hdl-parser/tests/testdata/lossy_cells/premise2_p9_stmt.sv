module t; logic clk = 0; logic [69:0] a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, b1, b2; logic [34:0] ch, cl; logic [69:0] mem [4]; int k;
  typedef struct packed { logic [39:0] a; logic [29:0] b; } s_t; s_t sa [2];
  wire [34:0] wh, wl; assign {wh, wl} = 'z;
  always @(posedge clk) a1 <= 'x;
  always_ff @(posedge clk) if (k == 0) a2 <= 'z; else a2 <= 'x;
  initial begin
    fork a3 = 'x; join
    a4 = 'z;
    k = 0; do begin a5 = 'x; k = k + 1; end while (k < 1);
    repeat (1) a6 = 'z;
    foreach (mem[i]) mem[i] = 'x;
    k = 2; mem[k] = 'z;
    {ch, cl} = 'x;
    sa[1] = '0; sa[1].a = 'z;
    b1 = 0; b1 = b1 <= 'x;
    for (logic [69:0] lv = 'z; k < 3; k++) a7 = lv;
    if (k > 100) a8 = 0; else a8 = 'x;
    #1 clk = 1; #1 a9 = 'z;
    #1;
    #1 $display("A a1=%h a2=%h a3=%h a4=%h a5=%h", a1, a2, a3, a4, a5);
    $display("B a6=%h m0=%h m2=%h c=%h%h sa1=%h", a6, mem[0], mem[2], ch, cl, sa[1]);
    $display("C b1=%h a7=%h a8=%h a9=%h w=%h%h", b1, a7, a8, a9, wh, wl);
  end
  initial #3 clk = 0;
  initial #100 $finish;
endmodule
