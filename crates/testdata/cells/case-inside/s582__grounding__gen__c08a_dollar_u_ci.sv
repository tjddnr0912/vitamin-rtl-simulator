module top;
  logic [3:0] v; int m;
  initial begin
    v = 4'd15; m = 9;
    case (v) inside
      [4'd12:$]: m = 1;
      [$:4'd2]: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd12; m = 9;
    case (v) inside
      [4'd12:$]: m = 1;
      [$:4'd2]: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd11; m = 9;
    case (v) inside
      [4'd12:$]: m = 1;
      [$:4'd2]: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd0; m = 9;
    case (v) inside
      [4'd12:$]: m = 1;
      [$:4'd2]: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd2; m = 9;
    case (v) inside
      [4'd12:$]: m = 1;
      [$:4'd2]: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd3; m = 9;
    case (v) inside
      [4'd12:$]: m = 1;
      [$:4'd2]: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    #10 $finish;
  end
endmodule
