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
  case (4'd3)
    q::h(2): begin : a initial #1 $display("arm=h"); end
    default: begin : d initial #1 $display("arm=def"); end
  endcase
  initial #2 $finish;
  initial #50 $finish;
endmodule
