package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x);
    $display("in h");
    h = 4'd3;
  endfunction
endpackage
module top;
  wire [31:0] w;
  assign w = {4'd0, {q::h(2){1'b1}}};
  initial begin #1 $display("w=%h", w); $finish; end
  initial #50 $finish;
endmodule
