package q;
  localparam int B1 = 7;
  function automatic logic [7:0] h(input int x);
    typedef enum logic [1:0] {B0, B1} e_t;
    logic [B1:0] t;
    t = '1;
    return t;
  endfunction
endpackage
module top;
  logic [7:0] v;
  initial begin v = q::h(0); $display("v=%h", v); #1 $finish; end
  initial #50 $finish;
endmodule
