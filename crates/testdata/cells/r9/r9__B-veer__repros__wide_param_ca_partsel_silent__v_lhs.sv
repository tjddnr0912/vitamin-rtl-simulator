typedef struct packed { logic [63:0] PAD; logic [7:0] HI; } prm_t;      // 72-bit packed-struct parameter type
module top #(parameter prm_t pt = 72'h0000000000000000_05);              // HI = 5
  logic [31:1]       f;
  logic [31:pt.HI+1] a;                                                   // [31:6]
  logic              sel;
  assign a[31:pt.HI+1] = sel ? f[31:6] : '0;                      // CA: part-selects bounded by a wide-parameter member
  initial begin
    sel = 1'b0; f = '0;
    #1 sel = 1'b1;
    #1 f = 31'h4000_0000;
    #1 $display("a=%h", a);
    $finish;
  end
endmodule
