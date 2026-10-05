module top;
  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    unique if (a == 1) t = 4'd1;
    begin
      logic [3:0] t = 4'd5;
    end
    return t;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin $display("P=%b", P); #1 $finish; end
endmodule
