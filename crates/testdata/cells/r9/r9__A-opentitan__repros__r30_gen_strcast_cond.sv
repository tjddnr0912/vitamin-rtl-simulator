module m #(parameter LfsrType = "GAL_XOR", parameter int W = 4) (output logic [3:0] o);
  if (64'(LfsrType) == 64'("GAL_XOR")) begin : g_gal
    assign o = 4'd1;
  end else if (64'(LfsrType) == 64'("FIB_XNOR")) begin : g_fib
    assign o = 4'd2;
  end else begin : g_none
    assign o = 4'd3;
  end
endmodule
module t;
  logic [3:0] a, b;
  m u0 (.o(a));
  m #(.LfsrType("FIB_XNOR")) u1 (.o(b));
  initial begin #1 $display("A a=%0d b=%0d", a, b); $finish; end
endmodule
