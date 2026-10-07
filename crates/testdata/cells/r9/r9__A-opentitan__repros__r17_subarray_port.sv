module c #(parameter int N = 2) (input logic [7:0] m [N], output logic [7:0] s); assign s = m[0] + m[1]; endmodule
module t;
  logic [7:0] mt [2][2]; logic [7:0] s0, s1;
  c u0 (.m(mt[0]), .s(s0));
  c u1 (.m(mt[1]), .s(s1));
  initial begin mt[0][0] = 1; mt[0][1] = 2; mt[1][0] = 10; mt[1][1] = 20; #1 $display("A s0=%0d s1=%0d", s0, s1); $finish; end
endmodule
