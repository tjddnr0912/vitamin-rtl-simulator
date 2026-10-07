module c (input logic [3:0] p, output logic [3:0] o); assign o = p + 4'd1; endmodule
module t;
  logic [3:0] pr [2]; logic [3:0] o0, o1; logic [3:0] a = 4'd5;
  assign pr = '{a, 4'd9};
  c u0 (.p(pr[0]), .o(o0));
  c u1 (.p(pr[1]), .o(o1));
  initial begin #1 $display("A o0=%0d o1=%0d", o0, o1); $finish; end
endmodule
