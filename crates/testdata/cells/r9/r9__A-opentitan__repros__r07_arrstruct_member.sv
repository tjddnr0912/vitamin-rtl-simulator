package p;
  typedef struct packed { logic [1:0] w; logic [1:0] r; } pol_t;
endpackage
module m (input p::pol_t [2:0] pol_i, input logic [1:0] sel [3], output logic [2:0] o);
  always_comb begin
    for (int unsigned k = 0; k < 3; k++) o[k] = |(pol_i[sel[k]].r & 2'b10);
  end
endmodule
module t;
  logic [2:0] o; logic [1:0] sel [3];
  m u (.pol_i(12'hA5C), .sel(sel), .o(o));
  initial begin sel[0] = 0; sel[1] = 1; sel[2] = 2; #1 $display("A o=%b", o); $finish; end
endmodule
