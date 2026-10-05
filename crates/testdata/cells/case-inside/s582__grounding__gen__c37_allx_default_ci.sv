module top;
  logic [3:0] v; int m;
  initial begin
    v = 4'bxxxx; m = 9;
    case (v) inside
      4'b????: m = 1;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'bzzzz; m = 9;
    case (v) inside
      4'b????: m = 1;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd3; m = 9;
    case (v) inside
      4'b????: m = 1;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    #10 $finish;
  end
endmodule
