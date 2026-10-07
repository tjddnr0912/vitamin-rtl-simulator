typedef struct packed { logic [63:0] PAD; logic [7:0] HI; } prm_t;
module top #(parameter prm_t pt = 72'h0000000000000000_05);
  logic [31:1] f;
  logic [5:0]  w;
  assign w = pt.HI + 1;           // the member value itself
  initial begin f = 31'h4000_0000; #1 $display("w=%0d sel=%h", w, f[31:pt.HI+1]); $finish; end
endmodule
