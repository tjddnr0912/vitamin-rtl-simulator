typedef struct packed { logic [63:0] PAD; logic [7:0] HI; logic [7:0] LO; } prm_t;
module top #(parameter prm_t pt = 80'h0000000000000000_05_02, parameter logic [71:0] WP = 72'h00_0000_0000_0000_0005);
  logic [31:0] f;
  logic [31:0] r1, r2, r3, r4, r5, r8, r9, r10, r11;
  logic [pt.HI-1:0] d1;
  wire  [31:0] w1 = f[pt.HI-1:0];
  function automatic logic [31:0] g(input logic [31:0] x); return x[pt.HI-1:0]; endfunction
  initial begin
    f = 32'hffff_ffff;
    r1 = f[pt.HI-1:0];          // 5 bits: 1f
    r2 = f[0 +: pt.HI+1];       // 6 bits: 3f
    r3 = {(pt.HI+1){1'b1}};     // 6 ones: 3f
    r4 = f[WP[7:0]+1:0];        // 7 bits 7f
    r5 = f[pt.HI*2:pt.LO];      // [10:2] 9 bits 1ff
    r8 = f[pt.HI:pt.LO];        // [5:2] 4 bits f  (no arithmetic)
    r9 = {pt.HI{1'b1}};         // 5 ones 1f (no arithmetic)
    r10 = g(f);                 // function body select: 1f
    d1 = '1; r11 = d1;          // declaration: 1f
    #1 $display("r1=%h r2=%h r3=%h r4=%h r5=%h r8=%h r9=%h r10=%h r11=%h w1=%h", r1, r2, r3, r4, r5, r8, r9, r10, r11, w1);
    $finish;
  end
endmodule
