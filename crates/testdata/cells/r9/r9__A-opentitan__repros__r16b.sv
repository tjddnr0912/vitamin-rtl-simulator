module c (input logic [3:0] p, output logic [3:0] o); assign o = p + 4'd1; endmodule
module t;
  localparam int N = 1;
  logic [3:0] pr [N]; logic [3:0] o0; logic [3:0] a = 4'd5;
  assign pr = '{a};
  for (genvar h = 0; h < N; h++) begin : g
    c u0 (.p(pr[h]), .o(o0));
  end
  initial begin #1 $display("A o0=%0d", o0); $finish; end
endmodule
