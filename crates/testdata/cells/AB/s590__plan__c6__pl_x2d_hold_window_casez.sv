module top;
  logic a; wire [1:0] w; logic [1:0] v;
  function logic [1:0] f(input logic x);
    $display("f t=%0t x=%b", $time, x);
    return {x, 1'b1};
  endfunction
  function logic [1:0] g(input logic [1:0] s);
    casez (s) 2'b0?: g = 2'd1; 2'b1?: g = 2'd2; default: g = 2'd3; endcase
  endfunction
  assign w = f(a);
  assign v = g(w);
  always @(v) $display("V t=%0t v=%b", $time, v);
  always @(posedge v[0]) $display("PV t=%0t v=%b", $time, v);
  initial begin
    a = 1;
    #1 $display("t=%0t w=%b v=%b", $time, w, v);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
