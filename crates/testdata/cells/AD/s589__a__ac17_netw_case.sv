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
  logic [q::h(2):0] v;
  initial begin v = '1; $display("v=%h b=%0d", v, $bits(v)); #1 $finish; end
  initial #50 $finish;
endmodule
