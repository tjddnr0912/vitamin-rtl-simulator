module top;
  typedef enum logic [1:0] {A, B, C} E;
  function automatic logic [31:0] f(input int a);
    E t;
    unique if (a == 1) t = 1;
    return {28'd0, t};
  endfunction
  localparam logic [31:0] P = f(2);
  initial begin $display("P=%h", P); #1 $finish; end
endmodule
