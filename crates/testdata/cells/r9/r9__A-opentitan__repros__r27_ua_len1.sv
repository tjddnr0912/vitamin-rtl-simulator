module c #(parameter int N = 1) (input logic [7:0] m [N], output logic [7:0] s); assign s = m[0] + 8'd1; endmodule
module p #(parameter int N = 1) (input logic [7:0] a [N], input logic sel, output logic [7:0] o [N]);
  always_comb begin
    case (sel)
      1'b1: o = a;
      default: o = '{default: 8'd0};
    endcase
  end
endmodule
module t;
  logic [7:0] s, s2; logic [7:0] x [1]; logic [7:0] y [1]; logic sel;
  c #(.N(1)) u (.m(x), .s(s));
  p #(.N(1)) up (.a(x), .sel(sel), .o(y));
  c #(.N(1)) u2 (.m(y), .s(s2));
  initial begin x[0] = 3; sel = 1; #1 $display("A s=%0d s2=%0d", s, s2); sel = 0; #1 $display("A s=%0d s2=%0d", s, s2); $finish; end
endmodule
