package q;
  function automatic int f(input int a);
    case (a)
      2: f = 3;
      default: f = 9;
    endcase
  endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  function automatic int f(input int a); return 3; endfunction
  wire [31:0] w;
  assign w = {q::h(1000){1'b1}};
  initial begin #1 $display("w=%h", w); $finish; end
  initial #50 $finish;
endmodule
