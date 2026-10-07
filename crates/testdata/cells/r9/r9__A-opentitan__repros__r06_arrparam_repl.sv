module m #(parameter int N = 3, parameter logic [1:0] V[N] = '{N{2'd1}}) (output logic [5:0] o);
  assign o = {V[0], V[1], V[2]};
endmodule
module t;
  logic [5:0] o;
  m u (.o(o));
  initial begin #1 $display("A o=%b", o); $finish; end
endmodule
