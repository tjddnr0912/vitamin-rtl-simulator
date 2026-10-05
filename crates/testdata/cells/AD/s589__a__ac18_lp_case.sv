package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x);
    case (x)
      2: h = 4'd3;
      default: h = 4'd1;
    endcase
  endfunction
endpackage
module top;
  localparam int P = q::h(2);
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #50 $finish;
endmodule
