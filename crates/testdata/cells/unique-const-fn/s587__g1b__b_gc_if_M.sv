module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  case (f(2))
    7: begin : g7
      initial begin #1 $display("gc=7"); $finish; end
    end
    default: begin : gd
      initial begin #1 $display("gc=other"); $finish; end
    end
  endcase
endmodule
